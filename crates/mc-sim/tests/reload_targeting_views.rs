//! What a swing can find follows a reload, not only an edit.
//!
//! # Why this exists beside the scenarios about an edit
//!
//! The world resolves what it holds into views the tick reads directly, and
//! there are **two** places either of them is written: one edit at a time, and
//! the whole registry replaced by content read while the game was running. The
//! scenarios about placing and breaking exercise the first and say nothing
//! whatever about the second. That the two bits cannot be written apart in a
//! wholesale replacement is an argument about how the code is shaped, and an
//! argument is not a witness.
//!
//! `reload_solidity_views.rs` is the same question asked about solidity and is
//! the file this one is modelled on. What is asked here is narrower and could
//! not be asked at all until what may be aimed at stopped being read off
//! solidity: an author says a block still stops a player and that no swing may
//! find it, and a ray has to start going through it without the world being
//! rebuilt.
//!
//! # The fixture, and what the two names are for
//!
//! A floor of grass, one cell of stone two blocks along the eye's own row, and
//! one cell of dirt four blocks along it. Both stand in row 11 — the eye is the
//! feet plus 1.62 blocks and the floor's top face is at y = 10 — and the row the
//! feet are in is empty along the ray. The two carry different names from each
//! other and from the floor, because a swing that took the wrong one of them
//! changes the same number of cells as one that took the right one, and only the
//! names say which went.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::{Simulation, seat};
use mc_sim::world::World;
use mc_world::world::{VoxelWorld, WorldPos};

use roots::{STONE_FILE, STONE_THAT_MAY_NOT_BE_AIMED_AT, adoption, shipped};
use support::chamber::{at, differences};
use support::{DIRT, GRASS, NOTHING, STONE, TestResult, content_registry, published_content};

/// Every cell at which a run differs from the world as declared.
type Changes = Vec<(WorldPos, String, String)>;

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// How far the floor runs on each horizontal axis: the whole column.
const FLOOR_SPAN: u32 = 16;

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// The stone two blocks along the ray, and the dirt four blocks along it.
const THE_STONE: WorldPos = at(10, EYE_ROW, 8);
const BEHIND_IT: WorldPos = at(12, EYE_ROW, 8);

/// Where the feet stand: on the floor's top face, at x = 8.0.
const LINED_UP: Vec3 = Vec3::new(8.0, 10.0, 8.5);

#[test]
fn a_swing_passes_through_a_block_a_reload_said_no_ray_may_stop_at() -> TestResult {
    let before = after_a_break(NoReload)?;
    let after = after_a_break(StoneNoLongerAimable)?;

    assert_eq!(
        (before, after),
        (
            vec![(THE_STONE, STONE.to_owned(), NOTHING.to_owned())],
            vec![(BEHIND_IT, DIRT.to_owned(), NOTHING.to_owned())]
        ),
        "the candidate says `{STONE}` still stops a player and that no swing may find it, and \
         changes nothing else. The world was not rebuilt and no cell of it was written, so the \
         only thing that can carry that answer to the walk is the wholesale replacement the \
         reload performs — a replacement that took the new registry and kept what the old one had \
         resolved about aiming leaves the swing taking the stone exactly as the first half does"
    );
    Ok(())
}

/// Whether the simulation is handed a candidate before the swing.
enum Reload {
    NoReload,
    StoneNoLongerAimable,
}

use Reload::{NoReload, StoneNoLongerAimable};

/// One tick over a fresh world asking for one break, optionally after a reload,
/// and what that did to the world compared with the same world as declared.
///
/// # Errors
///
/// Returns an error if the world does not build, if the candidate root cannot be
/// written or read, or if the reload was not admitted.
fn after_a_break(reload: Reload) -> Result<Changes, Box<dyn Error>> {
    let declared = two_blocks_in_line()?;
    let played = two_blocks_in_line()?;
    let content = published_content(played.registry())?;
    let mut simulation = seat(standing(), played, content).simulation;
    if let StoneNoLongerAimable = reload {
        admit(&mut simulation)?;
    }
    simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Break),
    });
    Ok(differences(&declared, simulation.world()))
}

/// Hands `simulation` a candidate root saying no ray may stop at stone, and
/// refuses unless it was admitted.
///
/// # Errors
///
/// Returns an error if the root cannot be written or read, or if the simulation
/// turned the candidate away — a scenario about what an admitted candidate does
/// to the walk is not a scenario about a candidate nobody admitted.
fn admit(simulation: &mut Simulation) -> Result<(), Box<dyn Error>> {
    let candidate = shipped()?.restating(STONE_FILE, STONE_THAT_MAY_NOT_BE_AIMED_AT)?;
    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        simulation,
        candidate.candidate()?,
    ));
    match answered {
        roots::Adoption::Accepted { .. } => Ok(()),
        other => Err(format!(
            "this scenario is about what an admitted candidate does to what a swing can find, and \
             the simulation answered {other:?} instead of admitting it"
        )
        .into()),
    }
}

/// A grass floor, one cell of stone two blocks along the eye's row, and one cell
/// of dirt four blocks along it.
fn two_blocks_in_line() -> Result<World, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let grass = BlockName::parse(GRASS)?;
    for z in 0..FLOOR_SPAN {
        for x in 0..FLOOR_SPAN {
            blocks.set_block(
                WorldPos {
                    x,
                    y: FLOOR_LAYER,
                    z,
                },
                &grass,
                &registry,
            )?;
        }
    }
    blocks.set_block(THE_STONE, &BlockName::parse(STONE)?, &registry)?;
    blocks.set_block(BEHIND_IT, &BlockName::parse(DIRT)?, &registry)?;
    Ok(World::new(blocks, registry)?)
}

/// A player standing still on the floor, facing along the row with a level view.
fn standing() -> PlayerState {
    PlayerState {
        position: LINED_UP,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}
