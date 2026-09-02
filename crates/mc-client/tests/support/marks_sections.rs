//! The client both marking suites read, and the drain that makes a reading
//! afterwards mean anything.
//!
//! # Why this is a module and not a copy in each suite
//!
//! `reload_marks_sections.rs` grew past the size a test file is allowed and was
//! split by the question each half answers — what a change to the **drawn**
//! picture leaves to mesh, and what a change to everything else leaves alone.
//! The three things below are what both halves need, and duplicating them would
//! be two launches that could drift apart while every reading over them stayed
//! green.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names the reload fixtures, which are themselves reached that way. A binary
//! including this must declare `mod support;`, the input harness,
//! [`crate::reload`], [`crate::reload_remesh`] and [`crate::reload_world`] as
//! well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;

use glam::Vec3;

use crate::input::InputHarness;
use crate::reload_remesh::{Marking, a_client_over, marked, require};
use crate::reload_world::{shipped_world, standing_at};
use crate::support::content_root;

/// Where the player stands: over the landmark pillar's top, in open air.
///
/// Nothing a tick does from here writes to a cell, so every mark either suite
/// reads is the reload's.
pub const IN_OPEN_AIR: Vec3 = Vec3::new(8.5, 70.0, 8.5);

/// A client playing the world it launches into, over the shipped content root.
///
/// # Errors
///
/// Returns the launch failure.
pub fn a_client_over_the_shipped_world() -> Result<InputHarness, Box<dyn Error>> {
    a_client_over(&content_root()?, standing_at(IN_OPEN_AIR), shipped_world)
}

/// Refuses unless the client has nothing outstanding to mesh.
///
/// **Both a guard and the reason the reading afterwards means anything**: a
/// launch that left sections marked would make every count in either suite the
/// reload's plus something else, and this is also the drain that leaves the set
/// empty for the reload to fill.
///
/// # Errors
///
/// Returns an error naming what the launch left outstanding.
pub fn require_nothing_outstanding(client: &mut InputHarness) -> Result<(), Box<dyn Error>> {
    let outstanding = marked(client);
    require(
        outstanding == Marking::NoSectionAtAll,
        format!(
            "this scenario reads what one reload left to be meshed, so the launch has to have left \
             nothing — and it left {outstanding:?}"
        ),
    )
}
