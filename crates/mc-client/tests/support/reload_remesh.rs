//! Which sections a reload left to mesh again, what a batch of them came to, and
//! what the worker made of one.
//!
//! # The dirty set is *taken*, so reading it twice reads it once
//!
//! `Session::take_remesh_work` drains. Every reading below therefore happens
//! exactly once per scenario and is held in a value, and a scenario wanting a
//! before and an after takes two separate readings around the thing it is judging.
//! A fixture that asked twice would find an empty set the second time and a
//! scenario reading it would call that "nothing was marked" — which is the
//! channel-blindness this spec has already paid for once, with the sign flipped.
//!
//! # Every verdict here is total, so an assertion against the good arm rejects
//! the answers that mean "there was nothing to look at"
//!
//! [`Marking`] has an arm for a set that is not the shipped world's, [`Collected`]
//! has one for nothing arriving at all, and [`Meshed`] has one for a batch that
//! never existed. `assert!(..is_some())` cannot tell any of those from the
//! property being asserted.
//!
//! # Three responsibilities, three files, one `#[path]`
//!
//! Building a client and reading what it serves lives here. What a reload left to
//! mesh again lives in [`marking`], what a collect came to in [`collecting`], and
//! what meshing a batch came to in [`meshing`] — each re-exported, so a scenario
//! names them exactly as it did when they shared a file.
//!
//! **The children are reached from here and not from the binaries.** Nine of them
//! include this module by `#[path]`, and a split that made each one name four paths
//! would be a line a tenth binary could omit — the same silent drop a filter that
//! matches nothing suffers, except that here it surfaces as a compile error in
//! somebody else's test rather than as a missing bound.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names a batch that carries its own registry and a three-armed `Remeshed`,
//! neither of which the implementation has written yet, exactly as
//! [`crate::reload`] and [`crate::reload_content`] do. A binary including this must
//! declare `mod support;`, the input harness and [`crate::reload_world`] as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

#[path = "reload_remesh/collecting.rs"]
mod collecting;
#[path = "reload_remesh/marking.rs"]
mod marking;
#[path = "reload_remesh/meshing.rs"]
mod meshing;

// Re-exported under the same allowance the module carries: a binary that drives
// only one of the three still links all three, so a glob it never names is the
// expected case rather than a stale import.
#[allow(unused_imports)]
pub use collecting::*;
#[allow(unused_imports)]
pub use marking::*;
#[allow(unused_imports)]
pub use meshing::*;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use mc_client::content::ContentView;
use mc_client::remesh::Retained;
use mc_core::block::BlockRegistry;
use mc_core::content::ContentSerial;
use mc_render::texture::TextureResolution;
use mc_sim::player::PlayerState;
use mc_sim::replay::world::FOOTPRINT_COLUMNS;
use mc_sim::world::World;
use mc_world::column::SECTIONS_PER_COLUMN;
use mc_world::world::VoxelWorld;
use winit::event::MouseButton;

use crate::input::InputHarness;
use crate::reload_world::{AIM_AT_THE_FAR_CELL, AIM_ON_TO_THE_NEAR_CELL, Edit, edit, playing};

/// One section of one column, as a value a scenario can compare and print.
pub type Section = (i32, i32, usize);

/// How many sections the shipped world stacks.
///
/// **Derived from the two declarations it is the product of** — four columns
/// square, sixteen sections each — so the 256 this phase's counts turn on appears
/// nowhere as a number somebody would have to keep in step by hand. A world built
/// to a different footprint fails the comparison loudly instead of agreeing over
/// fewer sections.
pub const EVERY_SECTION_OF_THE_SHIPPED_WORLD: usize =
    (FOOTPRINT_COLUMNS * FOOTPRINT_COLUMNS * SECTIONS_PER_COLUMN) as usize;

/// What a fixture says when it was asked to read something out of a client that
/// publishes no content at all.
pub const NOTHING_IS_SERVING: &str = "this fixture's client publishes no content, so there are no layers to draw with and no \
     serial a batch could have been drained under";

/// What a fixture says when a scenario needed a batch and the client had none.
pub const NOTHING_WAS_LEFT_TO_MESH: &str = "this scenario needs the client to have been left something to mesh again, and it was left \
     nothing at all";

/// A client playing the world `blocks_of` builds against the root at `root`, with
/// the player at `spawn`.
///
/// **No watcher.** The scenarios this serves hand the client a candidate through
/// its own door; the ones about what a report carries attach a watch instead.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if
/// the content declares no solid block at all.
pub fn a_client_over(
    root: &Path,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<InputHarness, Box<dyn Error>> {
    let (simulation, holding) = playing(root, spawn, blocks_of)?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok(client)
}

/// Breaks the cell the spawn's look meets first, and says what that did.
pub fn breaking_the_far_cell(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    client.click(MouseButton::Left);
    edit(client.edit())
}

/// Builds what the client is holding against the nearer of the two aimed-at
/// cells, from the spawn's own level look.
pub fn placing_over_the_near_cell(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_AT_THE_FAR_CELL);
    placing_over_the_near_cell_after_the_far_aim(client)
}

/// The same, for a client whose look is already aimed at the further cell.
///
/// **Raw counts accumulate into the pitch**, so a run that has already broken the
/// far cell must add only the difference between the two aims. Asking for the
/// whole of the nearer aim again would carry the look past the declared pitch limit
/// and clamp it, which is a third aim nothing derived.
pub fn placing_over_the_near_cell_after_the_far_aim(client: &mut InputHarness) -> Edit {
    client.move_pointer(0.0, AIM_ON_TO_THE_NEAR_CELL);
    client.click(MouseButton::Right);
    edit(client.edit())
}

/// The sections and the layers a launch over `blocks` would have retained.
///
/// **Meshed here through the world's own whole-world mesh**, which is the call a
/// launch makes, so the list a re-meshed section is spliced back into is the one
/// the product would have held.
///
/// # Errors
///
/// Returns an error if the world does not resolve against `registry`, or if it
/// cannot be meshed.
pub fn retained_at_launch(
    blocks: VoxelWorld,
    registry: Arc<BlockRegistry>,
    resolution: TextureResolution,
) -> Result<Retained, Box<dyn Error>> {
    Ok(Retained {
        meshed: World::new(blocks, registry)?.mesh()?,
        resolution,
    })
}

/// The layers a reader builds out of what `client` is publishing.
///
/// # Errors
///
/// Returns an error where the client publishes nothing.
pub fn resolution_serving(client: &InputHarness) -> Result<TextureResolution, Box<dyn Error>> {
    let published = client.content().ok_or(NOTHING_IS_SERVING)?;
    Ok(ContentView::of(&published.resolved).into_resolution())
}

/// The serial the content `client` is publishing was published under.
///
/// # Errors
///
/// Returns an error where the client publishes nothing.
pub fn serial_serving(client: &InputHarness) -> Result<ContentSerial, Box<dyn Error>> {
    Ok(client.content().ok_or(NOTHING_IS_SERVING)?.serial)
}

/// Fails with `explanation` unless `holds`.
///
/// A fixture that does not have the property an assertion rests on is a broken
/// fixture rather than a failed behaviour, and it says so before the assertion
/// runs.
///
/// # Errors
///
/// Returns `explanation` when `holds` is false.
pub fn require(holds: bool, explanation: String) -> Result<(), Box<dyn Error>> {
    if holds {
        Ok(())
    } else {
        Err(explanation.into())
    }
}
