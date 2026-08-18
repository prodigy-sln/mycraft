//! Reading a world back out of a save, and what a relaunch against changed
//! content makes of it.
//!
//! # This is how a scenario reads what a reload left, because it is the only way
//!
//! `Session` hands out no borrow of what it owns — no accessor for the simulation
//! and none for the world — which is a property of that type rather than an
//! oversight. So a scenario about what survived a swap reads it the way a player
//! would: by quitting, and by loading what was written. Everything in this module
//! goes through a save file on disk or through the launch that reopens one.
//!
//! **What a save is compared against is a declaration, never another run.** The
//! shipped world is regenerated from its declared seed and the small worlds are
//! built twice from the same description, so a swap that corrupted every cell the
//! same way on both sides has nothing to agree with.
//!
//! # Why this is a module of its own
//!
//! It was part of [`super::reload_world`] until that file reached the test-file
//! size limit, and the seam is the one the limit exists to force rather than a cut
//! made to satisfy a counter: that module is what a reload is **driven over** —
//! the world, where the player stands in it, and where they are looking — and this
//! is what is **read back afterwards**. Setting a run up and judging what it left
//! are two jobs, and the second one is the only half that touches a file.
//!
//! # Reached by `#[path]`, and only by the three suites that need it
//!
//! Like its sibling, it names types the implementation had not written when it was
//! authored, and a module declared in `support/mod.rs` is compiled into every
//! binary that says `mod support;`. A binary including this must declare
//! `mod support;` and `mod reload_world;` as well: the cells it reads and the
//! registry it relaunches against are that module's.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mc_client::launch::simulation_to_play;
use mc_client::startup::PreparationError;
use mc_core::block::BlockRegistry;
use mc_core::content::LayerAssignment;
use mc_core::id::BlockName;
use mc_render::window::Ending;
use mc_sim::persistence::Launching;
use mc_sim::player::BlockPos;
use mc_sim::simulation::{PublishedContent, Seated};
use mc_world::persistence::{Acceptance, load_world, requirements};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use crate::reload_world::{Cell, described_contents, inside};
use crate::support::reported;

/// What a cell the world does not reach is called. A third answer beside a block
/// and [`crate::reload_world::NOTHING`], never folded into either.
pub const OUTSIDE: &str = "no such cell";

/// The client's own two-component save layout, stated once.
const SAVE_LAYOUT: [&str; 2] = ["saves", "world.mcw"];

/// How many disagreements a comparison lists before it stops listing them.
const FIRST_FEW: usize = 5;

/// Where a save sits inside `directory`, in the layout a client looks for one in.
///
/// Neither component is created here: the writer makes the directories it needs.
#[must_use]
pub fn save_in(directory: &TempDir) -> PathBuf {
    let inside: PathBuf = SAVE_LAYOUT.iter().collect();
    directory.path().join(inside)
}

/// What the save at `save` holds at `at`, read back through the loader.
///
/// **This is the half that says there was a save to read.** A scenario asserting
/// what survived a reload is satisfied just as well by a save that was never
/// written or one the reader refused, and both read as an ordinary green unless
/// the refusal comes back as the answer.
///
/// # Errors
///
/// Returns the refusal, rendered, where the save could not be read.
pub fn saved_at(save: &Path, registry: &BlockRegistry, at: Cell) -> Result<String, String> {
    let loaded = load_world(save, registry, Acceptance::OnlyUnchangedBlocks)
        .map_err(|refused| refused.to_string())?;
    Ok(match inside(at) {
        Err(_) => OUTSIDE.to_owned(),
        Ok(position) => described(loaded.world.block_at(position)),
    })
}

/// How the world the save at `save` holds compares with `declared`, cell for cell
/// over everything `declared` reaches.
///
/// Returns how many cells were compared and the first few ways they disagreed.
/// **The count is fixture integrity and never the claim**: a walk that visited a
/// smaller world would agree over fewer cells and say nothing about the rest.
///
/// # Errors
///
/// Returns the refusal, rendered, where the save could not be read.
pub fn saved_against(
    save: &Path,
    registry: &BlockRegistry,
    declared: &VoxelWorld,
) -> Result<(usize, Vec<String>), String> {
    let loaded = load_world(save, registry, Acceptance::OnlyUnchangedBlocks)
        .map_err(|refused| refused.to_string())?;
    let mut compared = 0;
    let mut disagreements = Vec::new();
    for at in declared.extent().positions() {
        compared += 1;
        listed(
            &mut disagreements,
            disagreement_at(&loaded.world, declared, at),
        );
    }
    Ok((compared, disagreements))
}

/// How the two worlds disagree at `at`, if they disagree there at all.
fn disagreement_at(loaded: &VoxelWorld, declared: &VoxelWorld, at: WorldPos) -> Option<String> {
    let here = described(loaded.block_at(at));
    let there = described(declared.block_at(at));
    (here != there).then(|| disagreement(at, &here, &there))
}

/// Records `found` while the list is still short enough to be read.
///
/// A comparison over a million cells that reported every disagreement would bury
/// its own message; the count beside the list is what says how much was looked
/// at.
fn listed(into: &mut Vec<String>, found: Option<String>) {
    if let Some(sentence) = found.filter(|_| into.len() < FIRST_FEW) {
        into.push(sentence);
    }
}

/// What one save recorded about the blocks named in `names`, in that order.
///
/// The two halves a save records — what a block behaves like and what it looks
/// like — are exactly what "its declared fields" means to a relaunch, so this is
/// the reading that says whether a block an author never touched moved.
///
/// # Errors
///
/// Returns the refusal, rendered, where the save could not be read, or a sentence
/// naming a block the save does not carry: a world that stopped holding a block
/// is not a world a scenario about that block's declaration can be read from.
pub fn declared_for(save: &Path, names: &[&str]) -> Result<Vec<(String, u64, u64)>, String> {
    let recorded = declared_by(save)?;
    names.iter().map(|name| one_of(&recorded, name)).collect()
}

/// What every block the save at `save` needs was declared to be, in the order the
/// save's own table holds them.
///
/// # Errors
///
/// Returns the refusal, rendered, where the save could not be read.
pub fn declared_by(save: &Path) -> Result<Vec<(String, u64, u64)>, String> {
    Ok(requirements(save)
        .map_err(|refused| refused.to_string())?
        .blocks()
        .iter()
        .map(|block| {
            (
                block.name.as_str().to_owned(),
                block.behaviour.get(),
                block.appearance.get(),
            )
        })
        .collect())
}

/// What a launch that was not turned away is called, so that "it started" and a
/// refusal are the same kind of answer and a scenario compares one value.
pub const STARTED: &str = "it started a simulation";

/// What starting the client again against a content root and a save comes to.
pub type Launched = Result<(Seated, BlockName), PreparationError>;

/// The client starting again: the world at `save`, against the content the root
/// at `root` declares, with the player asked to accept nothing.
///
/// **Acceptance is refused rather than given**, because what these scenarios are
/// about is whether the player is *asked*: a launch that accepted changed blocks
/// would resume either way, and the question would have no answer.
///
/// **The content this launch publishes is the root's own**, read through the one
/// door rather than assembled out of the registry beside it. A relaunch really does
/// read a content root, so there is no reason for this fixture to state a second
/// answer about what that root resolves to — and a launch has spent no layers,
/// which is what `LayerAssignment::none` says at the call.
///
/// # Errors
///
/// Returns whichever reader refused the content root, before any launch is
/// attempted.
pub fn relaunch(root: &Path, save: &Path) -> Result<Launched, Box<dyn Error>> {
    let loaded = mc_sim::content::load(root, &LayerAssignment::none())?;
    Ok(simulation_to_play(
        save,
        Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::new(loaded.registry),
            content: PublishedContent::first(loaded.resolved, loaded.hud),
            accepting: Acceptance::OnlyUnchangedBlocks,
        },
    ))
}

/// What a player would be shown by that launch — [`STARTED`], or the refusal as
/// the client itself prints it.
///
/// **Printed through the client's own door and never composed here.** A refusal
/// is the only thing a player with a save they cannot open ever sees, and a
/// fixture that rendered one of its own would agree with itself while the client
/// printed a single sentence.
#[must_use]
pub fn how_it_went(launched: &Launched) -> String {
    let Err(turned_away) = launched else {
        return STARTED.to_owned();
    };
    reported(&Ending::failed(turned_away, &turned_away.way_out())).unwrap_or_else(|refused| {
        format!("what a player would be shown could not be written: {refused}")
    })
}

/// What the world that launch resumed holds at `at`.
///
/// # Errors
///
/// Returns [`STARTED`]'s opposite — the refusal, rendered — where the launch was
/// turned away, so a refusal and a wrong answer are the same failed assertion.
pub fn resumed_at(launched: &Launched, at: Cell) -> Result<String, String> {
    let Ok((simulation, _)) = launched else {
        return Err(how_it_went(launched));
    };
    let (x, y, z) = at;
    Ok(
        match simulation.simulation.world().block_at(BlockPos { x, y, z }) {
            None => OUTSIDE.to_owned(),
            Some(held) => described_contents(held),
        },
    )
}

/// One entry of what a save recorded, or a sentence saying the save has none.
fn one_of(recorded: &[(String, u64, u64)], name: &str) -> Result<(String, u64, u64), String> {
    recorded
        .iter()
        .find(|(held, _, _)| held == name)
        .cloned()
        .ok_or_else(|| {
            format!(
                "the save records nothing about {name}, so no cell of the world this fixture built \
                 holds it and a scenario about what its declaration says would be about a block \
                 that is not there"
            )
        })
}

/// How one cell disagrees, as one sentence.
fn disagreement(at: WorldPos, here: &str, there: &str) -> String {
    format!(
        "({x}, {y}, {z}) holds {here} where {there} was declared",
        x = at.x,
        y = at.y,
        z = at.z
    )
}

/// What `contents` holds, as text — the block's own name,
/// [`crate::reload_world::NOTHING`], or [`OUTSIDE`] where the world does not
/// reach.
fn described(contents: Result<Contents<&BlockName>, impl fmt::Display>) -> String {
    match contents {
        Err(_) => OUTSIDE.to_owned(),
        Ok(held) => described_contents(held),
    }
}
