//! The saves an entry is driven from, and what the launch that read them did
//! about the player.
//!
//! # Every fixture here is a real save file read through the shipped launch
//!
//! A player's position at entry is decided by `mc-sim` and the world it is
//! decided against is one a client loaded off disk, so an in-memory simulation
//! would take these scenarios off the only path where the played world's own
//! extent is actually supplied. So each fixture writes a save with
//! [`save_world`], and reads it back through
//! [`simulation_to_play`](mc_client::launch::simulation_to_play) with an
//! acceptance the client parsed out of a real argv.
//!
//! # A refusal and a wrong answer are the same failed assertion
//!
//! Every accessor reports `Err(the refusal, rendered)` where the launch was
//! turned away, so a scenario expecting a placement compares one value and never
//! asks `is_ok()` first. `super::persistence`'s own accessors follow the same
//! rule and are reused rather than restated.
//!
//! # Floats are compared as the integers they are
//!
//! "Where the player was put" means the same value and not a nearly equal one.
//! Every expected coordinate here is either a stored one — four bytes written and
//! the same four bytes read back — or a cell centre, which is a whole number plus
//! a half and exact in binary. There is no arithmetic between the two ends for a
//! tolerance to be about, and it is the form `clippy::float_cmp` has no quarrel
//! with.
//!
//! # The box's shape is restated, not imported
//!
//! [`cells_a_box_covers`] is an oracle over the launched world, so it may not call
//! the collision code the placement it is judging came out of — that would be
//! agreement between two copies of one decision. The width, the height and the
//! half-open `[v, v + 1)` rule are the specification's own
//! (`crates/mc-sim/src/player/collide.rs` states them once for the engine), and
//! they are written out again below as this fixture's declaration.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! A module declared in `support/mod.rs` is compiled into every binary that says
//! `mod support;`, and this one moves onto the admission door in the same commit
//! the whole suite does. Reached by path, only the suites that are about an entry
//! are in that window. A binary including this must also declare
//! `#[path = "support/persistence.rs"] mod persistence;` — the registry, the save
//! path and the launch accessors it builds on are that module's.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::sync::Arc;

use glam::Vec3;
use mc_client::launch::simulation_to_play;
use mc_client::startup::{PreparationError, acceptance_from};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::persistence::Launching;
use mc_sim::player::EYE_HEIGHT;
use mc_world::persistence::{SavedPlayer, save_world};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

use crate::persistence::{
    GROUND, Launched, declared, published_content, registry_of, save_in, with_the_replay_blocks,
};

/// How many blocks across one chunk column is.
pub const ACROSS: u32 = 16;

/// The one solid layer every fixture world lays down, and the row a player
/// standing on it has their feet in.
pub const FLOOR: u32 = 9;
pub const FEET_ROW: u32 = 10;

/// How far the clearing search looks, in blocks, on each axis it looks along.
///
/// A declaration of the search (`crates/mc-sim/src/world/clearing.rs`), restated
/// here rather than read out of the crate: a fixture reading the constant it
/// depends on agrees with a reach that moved.
pub const A_SEARCH_OF: i32 = 8;

/// How far the player's box reaches from the feet centre on x and z, and how tall
/// it is, in blocks.
///
/// The specification's own figures, and the crate's; see this module's header for
/// why they are restated rather than imported.
const HALF_WIDTH: f32 = 0.3;
const BOX_HEIGHT: f32 = 1.8;

/// The client's own argv, as a shell hands it over — the program's name first,
/// which is what the parse has to step past.
///
/// **One spelling and no second one.** A launch over a save whose blocks have
/// changed needs no argument now: loading is what a client does when it is told
/// nothing, so the fixtures here differ in the save they read and never in what
/// was typed. The argument that asks for the strict answer has no business in an
/// entry-clearing fixture, because a refused launch seats nobody.
pub const NO_ARGUMENT: [&str; 1] = ["mycraft"];

/// A save written to disk, and everything needed to read it back.
#[derive(Debug)]
pub struct ASave {
    /// The blocks that were written, for a scenario comparing the played world
    /// against them.
    pub blocks: VoxelWorld,
    /// The registry the save is read against, which is not always the one it was
    /// written against.
    pub registry: Arc<BlockRegistry>,
    /// Where the player was recorded standing, and which way they faced.
    pub recorded: SavedPlayer,
    /// Where the save file lives.
    pub directory: TempDir,
}

/// A registry declaring the fixture floor and the blocks the replay generator
/// places.
///
/// # Errors
///
/// Returns an error if a declaration or the registry refuses.
pub fn ground_registry() -> Result<Arc<BlockRegistry>, Box<dyn Error>> {
    Ok(Arc::new(with_the_replay_blocks(registry_of(vec![
        declared(GROUND, true)?,
    ])?)?))
}

/// An empty world `columns` square with one solid layer of `block` at [`FLOOR`].
///
/// # Errors
///
/// Returns an error if the name does not parse or the world refuses a write.
pub fn floor_of(
    registry: &BlockRegistry,
    columns: u32,
    block: &str,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(columns);
    let across = columns * ACROSS;
    let ground = BlockName::parse(block)?;
    for z in 0..across {
        for x in 0..across {
            blocks.set_block(WorldPos { x, y: FLOOR, z }, &ground, registry)?;
        }
    }
    Ok(blocks)
}

/// `blocks` with `block` written into every cell of `cells`.
///
/// # Errors
///
/// Returns an error if the name does not parse, if a cell lies outside the world,
/// or if the world refuses a write.
pub fn filling(
    blocks: &mut VoxelWorld,
    registry: &BlockRegistry,
    cells: &[(u32, u32, u32)],
    block: &str,
) -> Result<(), Box<dyn Error>> {
    let held = BlockName::parse(block)?;
    for (x, y, z) in cells.iter().copied() {
        blocks.set_block(WorldPos { x, y, z }, &held, registry)?;
    }
    Ok(())
}

/// Writes `blocks` and `recorded` to a save in a directory of its own, to be read
/// back against `registry`.
///
/// **`written_against` is a separate argument from `registry` on purpose.** A save
/// whose blocks were declared one way and is read against declarations that say
/// something else is the whole of one scenario here, and a helper taking one
/// registry could not express it.
///
/// # Errors
///
/// Returns an error if the directory cannot be made or the save cannot be
/// written.
pub fn written(
    blocks: VoxelWorld,
    written_against: &BlockRegistry,
    registry: Arc<BlockRegistry>,
    recorded: SavedPlayer,
) -> Result<ASave, Box<dyn Error>> {
    let directory = TempDir::new()?;
    save_world(&save_in(&directory), &blocks, recorded, written_against)?;
    Ok(ASave {
        blocks,
        registry,
        recorded,
        directory,
    })
}

/// What the client makes of `save` when it is started with `argv`.
///
/// # Errors
///
/// Returns an error if the content a launch publishes cannot be derived from the
/// registry.
pub fn resumed(save: &ASave, argv: &[&str]) -> Result<Launched, Box<dyn Error>> {
    Ok(simulation_to_play(
        &save_in(&save.directory),
        Launching {
            seed: mc_sim::REPLAY_SEED,
            registry: Arc::clone(&save.registry),
            content: published_content(&save.registry)?,
            accepting: acceptance_from(argv.iter().map(|argument| (*argument).to_string())),
        },
    ))
}

/// A player recorded standing at `feet`, facing `yaw` and looking `pitch`, in
/// radians.
#[must_use]
pub const fn recorded_at(feet: Vec3, yaw: f32, pitch: f32) -> SavedPlayer {
    SavedPlayer {
        position: [feet.x, feet.y, feet.z],
        yaw,
        pitch,
    }
}

/// A position as the integers its floats are.
#[must_use]
pub fn at(feet: Vec3) -> [u32; 3] {
    feet.to_array().map(f32::to_bits)
}

/// Where the eye of a player standing at `feet` sits, as the integers its floats
/// are.
///
/// The eye stands [`EYE_HEIGHT`] over the feet and nowhere else, which is the
/// crate's declaration; what is restated here is the arithmetic, so the camera a
/// snapshot carries is compared against the feet rather than against a second
/// call to the function that derived it.
#[must_use]
pub fn eye_over(feet: Vec3) -> [u32; 3] {
    at(feet + Vec3::Y * EYE_HEIGHT)
}

/// Which tick a launch published first, where it put the player in that snapshot,
/// and where that snapshot's camera stands — or the refusal it gave instead.
///
/// **The tick is part of the answer.** "The first snapshot the simulation
/// publishes" is a claim about the state before any intent has been submitted, and
/// a reading that arrived at the right position one tick later would satisfy a
/// comparison that left the tick out.
///
/// # Errors
///
/// Returns the rendered refusal where the launch was turned away.
pub fn first_snapshot(launched: &Launched) -> Result<(u32, [u32; 3], [u32; 3]), String> {
    let (playing, _) = launched.as_ref().map_err(PreparationError::to_string)?;
    let published = playing.simulation.latest();
    Ok((
        published.tick,
        published.player.position.to_array().map(f32::to_bits),
        published.camera.eye.map(f32::to_bits),
    ))
}

/// Every cell the box of a player standing at `feet` covers.
///
/// An oracle over a launched world, deliberately sharing no code with the search
/// whose answer it judges — see this module's header. A cell on the negative side
/// of an axis is dropped rather than reported, because a world has no position
/// for one and this is only ever asked about positions inside a fixture's world.
#[must_use]
pub fn cells_a_box_covers(feet: Vec3) -> Vec<(u32, u32, u32)> {
    let low = feet - Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH);
    let high = feet + Vec3::new(HALF_WIDTH, BOX_HEIGHT, HALF_WIDTH);
    (row(low.y)..=last_row(high.y))
        .flat_map(move |y| (row(low.z)..=last_row(high.z)).map(move |z| (y, z)))
        .flat_map(move |(y, z)| (row(low.x)..=last_row(high.x)).map(move |x| (x, y, z)))
        .filter_map(|(x, y, z)| {
            Some((
                u32::try_from(x).ok()?,
                u32::try_from(y).ok()?,
                u32::try_from(z).ok()?,
            ))
        })
        .collect()
}

/// Every cell the search may put a player's feet in around `feet`: the whole
/// horizontal reach, at their own row and the eight above it.
///
/// Derived from [`A_SEARCH_OF`] on every axis, so a fixture follows the declared
/// bound rather than a number of its own.
#[must_use]
pub fn the_cube_around(feet: Vec3) -> Vec<(i32, i32, i32)> {
    let (centre_x, centre_y, centre_z) = (row(feet.x), row(feet.y), row(feet.z));
    (0..=A_SEARCH_OF)
        .flat_map(|up| (-A_SEARCH_OF..=A_SEARCH_OF).map(move |along| (up, along)))
        .flat_map(|(up, along)| (-A_SEARCH_OF..=A_SEARCH_OF).map(move |across| (up, along, across)))
        .map(|(up, along, across)| (centre_x + across, centre_y + up, centre_z + along))
        .collect()
}

/// The cells of `cube` that lie inside a world `columns` square, as a world
/// spells a position.
///
/// **The filter is what makes a fixture near an edge buildable at all** — a write
/// past an edge is refused, and rightly — and it is also the shape of what the
/// search must refuse: the cells it drops are the ones nothing is loaded in.
#[must_use]
pub fn inside_a_world(cube: &[(i32, i32, i32)], columns: u32) -> Vec<(u32, u32, u32)> {
    let across = i32::try_from(columns * ACROSS).unwrap_or(i32::MAX);
    cube.iter()
        .copied()
        .filter(|(x, _, z)| (0..across).contains(x) && (0..across).contains(z))
        .filter_map(|(x, y, z)| {
            Some((
                u32::try_from(x).ok()?,
                u32::try_from(y).ok()?,
                u32::try_from(z).ok()?,
            ))
        })
        .collect()
}

/// `cells` with every cell of `left_clear` taken out of it.
#[must_use]
pub fn without(cells: &[(u32, u32, u32)], left_clear: &[(u32, u32, u32)]) -> Vec<(u32, u32, u32)> {
    cells
        .iter()
        .copied()
        .filter(|cell| !left_clear.contains(cell))
        .collect()
}

/// Refuses with `message` unless `premise` holds.
///
/// A fixture's premise is a constraint no assertion can enforce, so it is checked
/// where it is built and reported as an error rather than as a failed comparison
/// about something else.
///
/// # Errors
///
/// Returns `message` where the premise does not hold.
pub fn require(premise: bool, message: String) -> Result<(), Box<dyn Error>> {
    if premise { Ok(()) } else { Err(message.into()) }
}

/// The row a coordinate lies in.
fn row(coordinate: f32) -> i32 {
    coordinate.floor() as i32
}

/// The last row a box's upper corner reaches: a voxel fills `[v, v + 1)`, so a
/// face landing exactly on `v` stops one short of the voxel beginning there.
fn last_row(corner: f32) -> i32 {
    (corner.ceil() as i32).saturating_sub(1)
}
