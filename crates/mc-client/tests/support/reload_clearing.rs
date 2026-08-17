//! What a swap said about clearing the player, and where the player was one tick
//! later.
//!
//! # The player is read one tick AFTER the swap, and that is why this module exists
//!
//! `Simulation::adopt` publishes no tick of its own — `advance` is the only thing
//! that ever replaces the published pointer — so an assertion reading the snapshot
//! standing at the moment a candidate is taken up cannot see anything the swap did
//! to the player. Phase 1 shipped two scenarios green *and vacuous* for exactly
//! that reason, and both survived their named mutations. The clearing search writes
//! the player at the swap, so [`standing_of`] is only ever read from a snapshot a
//! further tick published.
//!
//! # Both verdicts are total
//!
//! [`Clearance`] and [`Standing`] each carry the arms that mean "there was nothing
//! to look at" — a client with no simulation, a run in which no boundary reported
//! anything, a client that has published nothing — so an assertion against one good
//! arm rejects every one of them for free. That is what `assert!(…is_some())` cannot
//! do.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names `mc_sim::world::Clearing` and a `clearing` field on
//! `ReloadReport::Accepted`, neither of which the implementation has written yet. A
//! module declared in `support/mod.rs` is compiled into every binary that says
//! `mod support;`, which would leave the whole crate unable to build for the whole
//! of that window. A binary including this must declare `mod support;`, the input
//! harness and [`crate::reload_watch`] as well.
//!
//! The worlds these scenarios are driven over, and the oracle for what is clear in
//! one, are [`crate::reload_trap`]. The seam is the one the size limit forced and it
//! is this file's own header: here is what a reading *is*, next door is what a
//! reload is driven *over*.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Instant;

use glam::Vec3;
use mc_client::session::reload::ReloadReport;
use mc_sim::reload::ReloadRefusal;
use mc_sim::simulation::{Accepted, SimSnapshot};
use mc_sim::world::Clearing;

use crate::input::InputHarness;
use crate::reload_watch::{may_cross_another, pause_between_boundaries};

/// A position, as the integers those floats are.
///
/// **Compared as bits and never with a tolerance.** What these scenarios ask is
/// *where* a swap put the player, and two of them ask whether it moved them at all —
/// a tolerance answers "near enough" to a question about "not". Every expectation
/// is a half, an eighth or a whole, all of which are exact in binary.
pub type Feet = [u32; 3];

/// `feet` as the integers those floats are.
#[must_use]
pub fn feet_at(feet: Vec3) -> Feet {
    feet.to_array().map(f32::to_bits)
}

/// What a swap said about clearing the player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clearance {
    /// No cell the player's box overlaps became solid, so nothing moved them.
    NoMoveNeeded,
    /// The player was put here.
    MovedTo(Feet),
    /// Nothing clear was found inside this many blocks.
    NoClearSpaceWithin { blocks: u32 },
    /// The world holds these blocks and the candidate declares none of them, in the
    /// order the refusal named them.
    BlocksTheWorldHolds(Vec<String>),
    /// A refusal this suite does not recognise, rendered.
    RefusedOtherwise(String),
    /// There was no simulation to hand the candidate to.
    NoSimulation,
    /// No boundary reported anything before the run gave up.
    NothingReported,
}

/// The verdict `clearing` is, as a [`Clearance`].
///
/// Matched exhaustively rather than with a fallback: three arms are the whole of the
/// enumerated verdict the architecture declares, and a fourth would be a change to
/// that contract rather than a refusal a later phase adds.
#[must_use]
pub fn clearance(clearing: &Clearing) -> Clearance {
    match clearing {
        Clearing::Unneeded => Clearance::NoMoveNeeded,
        Clearing::MovedTo(feet) => Clearance::MovedTo(feet_at(*feet)),
        Clearing::NoClearSpaceWithin { blocks } => {
            Clearance::NoClearSpaceWithin { blocks: *blocks }
        }
    }
}

/// What a client's answer to a candidate said about clearing the player.
#[must_use]
pub fn clearance_of(answered: Option<Result<Accepted, ReloadRefusal>>) -> Clearance {
    match answered {
        None => Clearance::NoSimulation,
        Some(Ok(accepted)) => clearance(&accepted.clearing),
        Some(Err(refused)) => refusal(&refused),
    }
}

/// What one refusal says, as an arm of [`Clearance`].
///
/// One question and a fallback rather than a `match`, for [`crate::reload`]'s
/// reason: `ReloadRefusal` grows variants, and a fallback that is reachable by
/// construction goes on *reporting* which refusal arrived instead of breaking this
/// file's compilation.
fn refusal(refused: &ReloadRefusal) -> Clearance {
    if let ReloadRefusal::BlocksTheWorldHolds { blocks } = refused {
        return Clearance::BlocksTheWorldHolds(
            blocks
                .iter()
                .map(|block| block.as_str().to_owned())
                .collect(),
        );
    }
    Clearance::RefusedOtherwise(refused.to_string())
}

/// A move to `feet`, for a scenario to compare against.
#[must_use]
pub fn moved_to(feet: Vec3) -> Clearance {
    Clearance::MovedTo(feet_at(feet))
}

/// A refusal naming these blocks, in this order.
#[must_use]
pub fn holding_blocks_it_does_not_declare(blocks: &[&str]) -> Clearance {
    Clearance::BlocksTheWorldHolds(blocks.iter().map(|block| (*block).to_owned()).collect())
}

/// Where the player stood and how fast they were moving, as the integers those
/// floats are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    At {
        feet: Feet,
        velocity: Feet,
    },
    /// Nothing has been published, which is a client with no world rather than a
    /// player who stayed put.
    NothingPublished,
}

/// What `published` says about the player.
#[must_use]
pub fn standing_of(published: Option<Arc<SimSnapshot>>) -> Standing {
    match published {
        None => Standing::NothingPublished,
        Some(snapshot) => Standing::At {
            feet: feet_at(snapshot.player.position),
            velocity: feet_at(snapshot.player.velocity),
        },
    }
}

/// A player at rest with their feet at `feet`, for a scenario to compare against.
///
/// **The velocity is part of every comparison and not only the one scenario that is
/// about it.** Every destination these scenarios name is a cell whose own floor
/// holds the player up, so the tick after the swap ends at rest either way — except
/// where the player was moving *upward* when the swap happened, which is the one
/// case a preserved velocity survives that tick.
#[must_use]
pub fn at(feet: Vec3) -> Standing {
    Standing::At {
        feet: feet_at(feet),
        velocity: feet_at(Vec3::ZERO),
    }
}

/// Crosses tick boundaries until one reports what it made of the content root, and
/// says what it said about clearing the player.
///
/// **The verdict is read out of [`ReloadReport`] and never out of a call a scenario
/// makes**, which is the whole of what requiring the system to *report* it means: a
/// verdict computed and dropped satisfies nothing, and the report is where it crosses
/// out of the client's
/// core on its way to the one place that prints it for a person.
///
/// The patience is [`crate::reload_watch`]'s, so there is one statement of it rather
/// than a second number beside it.
pub fn until_cleared(client: &mut InputHarness) -> Clearance {
    let started = Instant::now();
    while may_cross_another(started) {
        client.tick();
        match client.take_reload_report() {
            None => pause_between_boundaries(),
            Some(ReloadReport::Refused(said)) => return Clearance::RefusedOtherwise(said),
            Some(ReloadReport::Accepted { clearing, .. }) => return clearance(&clearing),
        }
    }
    Clearance::NothingReported
}
