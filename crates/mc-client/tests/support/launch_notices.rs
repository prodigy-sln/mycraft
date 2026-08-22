//! The notices a launch writes about the save it read — what entry did about a
//! player it found inside solid rock, and which of that save's blocks no longer
//! behave as they did.
//!
//! # Why these live apart from the refusals
//!
//! `printed_refusals.rs` produces the lines a *content root* is turned away with,
//! and it grows one refusal at a time. These are the other kind: **nothing here is
//! a refusal at all.** A launch that meets one of them proceeds — the player is in
//! their world by the time they read it — so what a page quoting one promises is
//! not "this is what stops you" but "this is what you are told". The recogniser in
//! `documented_refusals.rs` does not know the difference and must not: it matches
//! the prefix the reporting writes, so whatever a page shows under it is held to
//! what a run produces either way.
//!
//! The split is by that responsibility rather than by line count, which is the
//! same reason the per-facing and built-set groups have modules of their own.
//!
//! # Every line here is composed by production over a verdict a real launch reached
//!
//! Nothing is written out. A fixture decides which world is read and which save is
//! loaded, and a premise refuses to hand back a line unless the launch arrived at
//! the answer that line is about — because a sentence spelled out in a fixture is a
//! third copy of somebody's belief about the program, and a page held to it would
//! be agreeing with the fixture rather than with the client.

// Each binary that includes this drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use glam::Vec3;
use mc_client::notice::{changed_blocks, entering};
use mc_sim::world::Clearing;

use crate::entry;
use crate::persistence::GROUND;
use crate::printed_refusals::normalised;
use crate::support;

/// Every line a launch writes about the save it read: the two an entry can write
/// about where it put the player, and the one naming blocks that no longer behave
/// as the save recorded them.
///
/// **Three launches and not one.** One launch answers one thing about one player,
/// and the two entry answers differ in the world that was read rather than in
/// anything asked for.
///
/// # Errors
///
/// Returns an error if a fixture cannot be built or written, if a launch was
/// turned away, or if a launch arrived at an answer the line it is asked for is
/// not about.
pub fn launch_notices() -> Result<Vec<String>, Box<dyn Error>> {
    Ok(vec![
        an_entry_moving_a_trapped_player()?,
        an_entry_with_nowhere_to_put_them()?,
        a_launch_over_a_save_whose_block_behaves_differently()?,
    ])
}

/// How many chunk columns square the world an entry is driven in is.
///
/// Three columns is forty-eight blocks across, which holds the whole seventeen
/// blocks of the search cube around the position below with room to spare — the
/// premise that keeps "nothing within eight blocks is clear" a sentence about
/// solid ground rather than about the edge of the world.
const THREE_COLUMNS: u32 = 3;

/// Where the save records the player an entry has to answer for: inside solid
/// rock, feet a quarter of a block above the row they are in.
const TRAPPED_FEET: Vec3 = Vec3::new(16.5, entry::FEET_ROW as f32 + 0.25, 16.5);

/// The two cells the one way out occupies, four blocks along −x and −z of the
/// trap, and the only position inside the search cube a standing player fits.
///
/// **Where the way out is decides what a page has to quote**, since the sentence
/// carries the destination — so it is a cell picked for the coordinate it centres
/// on rather than the first one that would do.
const THE_WAY_OUT_CELLS: [(u32, u32, u32); 2] =
    [(12, entry::FEET_ROW, 12), (12, entry::FEET_ROW + 1, 12)];

/// What a player entry had to move out of solid rock reads.
///
/// **Composed by [`mc_client::notice::entering`] over a verdict a real launch came
/// to.** Nothing about the sentence is decided here: the fixture decides only which
/// of the two answers the launch arrives at, and the premise below refuses to hand
/// back a line if it arrived at the other one.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or written, if the launch was
/// turned away, or if entry did anything other than move the player — a run that
/// moved nobody has none of this line for a page to quote.
fn an_entry_moving_a_trapped_player() -> Result<String, Box<dyn Error>> {
    let clearing = what_entry_did(&a_wedge_leaving(&THE_WAY_OUT_CELLS)?)?;
    entry::require(
        matches!(clearing, Clearing::MovedTo(_)),
        format!(
            "a page quoting where entry put a player needs a run that put one somewhere, and the \
             launch over the trap at {TRAPPED_FEET:?} answered {clearing:?}"
        ),
    )?;
    said_at_entry(clearing)
}

/// What a player entry could find nowhere to put reads.
///
/// The same wedge with its one way out filled in, so the pair a page carries differs
/// in the world the launch read and not in what this module asked for.
///
/// # Errors
///
/// Returns an error if the fixture cannot be built or written, if the launch was
/// turned away, or if entry found somewhere to put the player after all.
fn an_entry_with_nowhere_to_put_them() -> Result<String, Box<dyn Error>> {
    let clearing = what_entry_did(&a_wedge_leaving(&[])?)?;
    entry::require(
        matches!(clearing, Clearing::NoClearSpaceWithin { .. }),
        format!(
            "a page quoting the answer for a player nothing could be done for needs a run that \
             found nowhere to put one, and the launch over the wedge at {TRAPPED_FEET:?} answered \
             {clearing:?}"
        ),
    )?;
    said_at_entry(clearing)
}

/// The save written before this repository's blocks were Luau, relative to the
/// repository root.
///
/// **Never regenerated**, which is what makes it an oracle: it predates the
/// declarations it is read against, so a line produced over it is a line a real
/// disagreement produced rather than one a fixture arranged.
const OLDER_SAVE: [&str; 5] = [
    "crates",
    "mc-world",
    "tests",
    "fixtures",
    "world_saved_against_the_toml_declarations.mcw",
];

/// What a launch over a save whose block behaves differently writes.
///
/// **Composed by [`mc_client::notice::changed_blocks`] over a list a real load
/// produced**, on the same terms as the two entry notices above: the fixture
/// decides which save is read and never what the sentence says. The save is the
/// committed pre-Luau one and the content is the shipped root, so the block named
/// is whichever one those two genuinely disagree about.
///
/// # Errors
///
/// Returns an error if the repository or the shipped content cannot be read, if
/// the launch was turned away, or if the load reported no changed block at all —
/// a run that found nothing has no line of this for a page to quote.
fn a_launch_over_a_save_whose_block_behaves_differently() -> Result<String, Box<dyn Error>> {
    let registry = Arc::new(support::content_registry()?);
    let launched = mc_client::launch::simulation_to_play(
        &support::repository_root()?.join(OLDER_SAVE.iter().collect::<PathBuf>()),
        mc_sim::persistence::Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&registry),
            content: support::published_content(&registry)?,
            accepting: mc_world::persistence::Acceptance::ChangedBlocksToo,
        },
    );
    let (seated, _) = launched.map_err(|refused| refused.to_string())?;
    changed_blocks(&seated.changed)
        .map(|said| normalised(&said))
        .ok_or_else(|| {
            "this producer needs a save the shipped content disagrees with about a block's              behaviour, and the load reported none. There is no line for a page to quote"
                .into()
        })
}

/// The line the client writes about `clearing` when a player enters.
///
/// # Errors
///
/// Returns an error where entry writes nothing at all, which is what every ordinary
/// launch does and is no page's business.
fn said_at_entry(clearing: Clearing) -> Result<String, Box<dyn Error>> {
    entering(clearing)
        .map(|said| normalised(&said))
        .ok_or_else(|| {
            format!("entry wrote nothing about {clearing:?}, so no line of it can be quoted").into()
        })
}

/// What the launch that read `save` did about where the player stands.
///
/// # Errors
///
/// Returns an error if the launch was turned away, in which case no player entered
/// and there is nothing for a page to quote.
fn what_entry_did(save: &entry::ASave) -> Result<Clearing, Box<dyn Error>> {
    let launched = entry::resumed(save, &entry::NO_ARGUMENT)?;
    let (seated, _) = launched.map_err(|refused| refused.to_string())?;
    Ok(seated.clearing)
}

/// A save recording a player inside solid rock, with every position the search may
/// consider filled in except the cells of `left_clear`.
///
/// **A wedge and never an edge.** The world is wide enough to hold the whole search
/// cube, so what leaves a player nowhere to go is solid ground rather than the world
/// running out — and that premise is checked here rather than assumed, because a
/// fixture that had drifted into an edge would still produce a sentence and a page
/// would still quote it.
///
/// # Errors
///
/// Returns an error if the world cannot be built or written, or if the cube does not
/// lie entirely inside the world.
fn a_wedge_leaving(left_clear: &[(u32, u32, u32)]) -> Result<entry::ASave, Box<dyn Error>> {
    let registry = entry::ground_registry()?;
    let mut blocks = entry::floor_of(&registry, THREE_COLUMNS, GROUND)?;
    let cube = entry::the_cube_around(TRAPPED_FEET);
    let inside = entry::inside_a_world(&cube, THREE_COLUMNS);
    entry::require(
        inside.len() == cube.len(),
        format!(
            "this fixture needs every position the search may look at to be inside the world, or \
             it is about an edge rather than about a wedge — {outside} of the {total} positions \
             around {TRAPPED_FEET:?} lie outside a world {THREE_COLUMNS} columns square",
            outside = cube.len() - inside.len(),
            total = cube.len()
        ),
    )?;
    entry::filling(
        &mut blocks,
        &registry,
        &entry::without(&inside, left_clear),
        GROUND,
    )?;
    entry::written(
        blocks,
        &registry,
        Arc::clone(&registry),
        entry::recorded_at(TRAPPED_FEET, 0.0, 0.0),
    )
}
