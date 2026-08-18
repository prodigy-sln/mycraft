//! The worlds a reload traps a player in, the clients that play them, and an oracle
//! for what is clear in one.
//!
//! # Every world here is named against a real content root
//!
//! The registry a world is built from is the one `mc_sim::content::load` hands back
//! for a content root on disk, never one assembled in Rust — [`crate::reload_world`]'s
//! rule, inherited unchanged. `base:water` is the one shipped block whose own
//! declaration calls it not solid, so it is the only block an author can make solid
//! without also having to place something new, which is why every scenario is driven
//! with the same one-line edit ([`water_that_is_solid`]).
//!
//! # The oracle shares no code with the search it grades
//!
//! [`overlap_at`] walks the box over the world the fixture declared, using the
//! declared `0.6 × 1.8 × 0.6` box and the half-open `[v, v + 1)` rule, and asks a
//! *named* list of blocks which are solid. It reaches none of `collide::overlaps`,
//! none of `SolidVoxels` and none of the search. What that buys is the premise of
//! every scenario, asserted rather than described: that the reload is what trapped
//! the player, and that a position a scenario names as available really is clear. A
//! guard that asked the subject's own predicate would agree with it whatever it did.
//!
//! # The one place a count of positions appears, and why it is not an expectation
//!
//! [`require_nothing_clear_within_the_search`] walks the cube the search walks in
//! order to say that a world really has nowhere clear in it. That is a statement
//! about the **fixture**. Nothing in these scenarios asserts a count of positions the
//! subject tested: the spec's ceiling is `17³ = 4 913` and the search spends
//! `9 × 17 × 17 = 2 601` of it, and an assertion on either would redden against a
//! conforming implementation.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! For the reason [`crate::reload_clearing`] is, and because it is that module's
//! other half: the size limit forced the split and the seam is each header's own —
//! here is what a reload is driven *over*, next door is what a reading *is*. A binary
//! including this must declare `mod support;`, the input harness,
//! [`crate::reload`], [`crate::reload_watch`] and [`crate::reload_world`] as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_sim::player::PlayerState;
use mc_sim::reload::ContentReload;
use mc_sim::simulation::SimSnapshot;
use mc_world::section::{Contents, SECTION_SIZE};
use mc_world::world::{VoxelWorld, WorldPos};

use crate::input::InputHarness;
use crate::reload::{DIRT, Declaration, GRASS, STONE, WATER};
use crate::reload_watch::{Reports, watch};
use crate::reload_world::{Cell, FLOOR, inside, playing, registry_of};
use crate::support::content::ContentRoot;

/// How far the clearing search may look, in blocks.
///
/// The declared bound is eight blocks, and it is written out here rather than read
/// from whatever constant the implementation declares: that constant is on
/// the other side of a comparison: the verdict under test reports the distance it
/// looked as a number, and an expectation assembled from the implementation's own
/// constant would read back whatever that became.
pub const A_SEARCH_OF: u32 = 8;

/// How far the player's box reaches from the feet centre on x and z, and how tall it
/// is — the declared `0.6 × 1.8 × 0.6`.
///
/// **Stated here rather than borrowed, deliberately.** `collide.rs` declares the same
/// two numbers for the physics and the search under test reads them from there; if
/// the two ever disagree, the guards below fail. An oracle that called the subject's
/// own predicate could not give that answer.
const HALF_WIDTH: f32 = 0.3;
const BOX_HEIGHT: f32 = 1.8;

/// The row the floor these worlds carry is laid in, the row a player standing on it
/// has their feet in, and the row their head reaches.
///
/// A box 1.8 blocks tall standing on that floor's top face occupies exactly two rows,
/// which is what makes clearance a question about two cells rather than about a
/// column of them.
pub const FLOOR_ROW: i32 = FLOOR;
pub const FEET_ROW: i32 = FLOOR + 1;
pub const HEAD_ROW: i32 = FLOOR + 2;

/// How high the feet of a player standing on that floor are.
pub const ON_THE_FLOOR: f32 = FEET_ROW as f32;

/// The blocks the shipped content calls solid, and the same list once a candidate has
/// declared `base:water` solid.
///
/// **Listed rather than read back**, for the reason [`crate::reload_watch`] lists the
/// four shipped blocks: a fixture that discovered them would go on passing over a
/// root that had stopped declaring one, and every guard below is a statement about
/// which cells a *named* content set makes solid.
pub const SOLID_WHILE_SERVING: [&str; 3] = [DIRT, GRASS, STONE];
pub const SOLID_ONCE_WATER_IS: [&str; 4] = [DIRT, GRASS, STONE, WATER];

/// A candidate's declaration of `base:water`, solid.
#[must_use]
pub fn water_that_is_solid() -> Declaration {
    Declaration::of(WATER).solid(true)
}

/// Which cells of a world a player's box overlaps that something calls solid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlap {
    /// The box overlaps nothing solid.
    Clear,
    /// It overlaps these cells.
    Overlapping(Vec<Cell>),
}

/// Which cells of `blocks` the box a player at `feet` carries overlaps that hold one
/// of `solid`.
///
/// See this module's header: an oracle over the declared world, sharing no code with
/// the search it grades.
#[must_use]
pub fn overlap_at(blocks: &VoxelWorld, solid: &[&str], feet: Vec3) -> Overlap {
    let touching: Vec<Cell> = box_cells(feet)
        .filter(|cell| holds_one_of(blocks, *cell, solid))
        .collect();
    if touching.is_empty() {
        Overlap::Clear
    } else {
        Overlap::Overlapping(touching)
    }
}

/// Every cell the box a player at `feet` carries reaches.
///
/// A voxel fills `[v, v + 1)`, so the cells an interval `[min, max]` touches run
/// `floor(min)` up to and including `ceil(max) − 1`.
fn box_cells(feet: Vec3) -> impl Iterator<Item = Cell> {
    let least = floor_cell(feet - Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH));
    let most = ceil_cell(feet + Vec3::new(HALF_WIDTH, BOX_HEIGHT, HALF_WIDTH));
    (least.1..=most.1).flat_map(move |y| {
        (least.2..=most.2).flat_map(move |z| (least.0..=most.0).map(move |x| (x, y, z)))
    })
}

/// The cell a box's lower corner lies in.
fn floor_cell(corner: Vec3) -> Cell {
    (
        corner.x.floor() as i32,
        corner.y.floor() as i32,
        corner.z.floor() as i32,
    )
}

/// The last cell a box's upper corner reaches.
fn ceil_cell(corner: Vec3) -> Cell {
    (
        corner.x.ceil() as i32 - 1,
        corner.y.ceil() as i32 - 1,
        corner.z.ceil() as i32 - 1,
    )
}

/// Whether the cell at `cell` holds one of `solid`.
///
/// A cell the world does not reach holds nothing solid, which is the answer the
/// simulation's own collision view gives for the same position. The two arms are
/// folded because this oracle has nothing to do about the difference, and the worlds
/// below are built wide enough that no scenario depends on it.
fn holds_one_of(blocks: &VoxelWorld, cell: Cell, solid: &[&str]) -> bool {
    let Ok(at) = inside(cell) else {
        return false;
    };
    match blocks.block_at(at) {
        Ok(Contents::Holds(name)) => solid.contains(&name.as_str()),
        Ok(Contents::Empty) | Err(_) => false,
    }
}

/// Refuses with `said` unless `held`.
///
/// # Errors
///
/// Returns `said` where `held` is false.
pub fn require(held: bool, said: String) -> Result<(), Box<dyn Error>> {
    if held { Ok(()) } else { Err(said.into()) }
}

/// Refuses unless the reload is what traps the player: the box at `feet` overlaps
/// nothing solid under the content serving, and something solid once water is.
///
/// **The premise of every scenario about a player who has to be moved, and it is two
/// halves.** A fixture whose player already stood in something solid would be about a
/// world that was wedged before anybody edited anything; one whose player the
/// candidate never reaches would be about a reload with no work to do. Either way a
/// clearing verdict would arrive that a reader would take at face value.
///
/// # Errors
///
/// Returns an error naming which half failed and which cells it found.
pub fn require_the_reload_traps(blocks: &VoxelWorld, feet: Vec3) -> Result<(), Box<dyn Error>> {
    let serving = overlap_at(blocks, &SOLID_WHILE_SERVING, feet);
    let once_water_is = overlap_at(blocks, &SOLID_ONCE_WATER_IS, feet);
    require(
        serving == Overlap::Clear,
        format!(
            "this scenario needs the reload to be what traps the player, so their box has to \
             overlap nothing solid under the content serving — and it overlaps {serving:?}"
        ),
    )?;
    require(
        matches!(once_water_is, Overlap::Overlapping(_)),
        format!(
            "this scenario needs a candidate that makes solid a cell the player's box overlaps, \
             and once `base:water` is solid that box overlaps {once_water_is:?}"
        ),
    )
}

/// Refuses unless the box at `feet` overlaps nothing solid once water is solid — the
/// premise of every scenario that names a position the search could have put the
/// player in.
///
/// # Errors
///
/// Returns an error naming the cells it found instead.
pub fn require_a_clear_position_at(blocks: &VoxelWorld, feet: Vec3) -> Result<(), Box<dyn Error>> {
    let once_water_is = overlap_at(blocks, &SOLID_ONCE_WATER_IS, feet);
    require(
        once_water_is == Overlap::Clear,
        format!(
            "this scenario names {feet:?} as a position the search could have put the player in, \
             and once `base:water` is solid a box there overlaps {once_water_is:?}"
        ),
    )
}

/// Refuses unless the box at `feet` overlaps nothing solid either way — the premise of
/// the scenario about a cell the player's box does *not* overlap.
///
/// # Errors
///
/// Returns an error naming the cells it found.
pub fn require_the_reload_misses(blocks: &VoxelWorld, feet: Vec3) -> Result<(), Box<dyn Error>> {
    let once_water_is = overlap_at(blocks, &SOLID_ONCE_WATER_IS, feet);
    require(
        once_water_is == Overlap::Clear,
        format!(
            "this scenario is about a reload that makes solid a cell the player's box does not \
             overlap, so nothing it makes solid may lie in that box — and it overlaps \
             {once_water_is:?}"
        ),
    )
}

/// Refuses unless a search wrongly reached from the refusal path would have had
/// somewhere to move the player, **and** the candidate would have made solid one
/// further cell their box overlaps.
///
/// **The premise a scenario about a refused candidate turns on, and it is three
/// halves.** The candidate has to be one that could have made a further cell of the
/// box solid, or it is not the candidate the scenario describes. The box has to
/// already stand in something solid under the content *serving*, because a search
/// called on the refusal path runs against the solidity the world still has — and the
/// search has to have somewhere to put them, or that call answers "nothing within
/// eight blocks is clear" and moves nobody anyway. Miss any one and the named
/// mutation leaves the scenario green while proving nothing. **Measured, not
/// reasoned: with the stone cell removed and this guard replaced by
/// [`require_the_reload_traps`], the mutation left the scenario passing.** Measured
/// again after entry clearing shipped and the fixture was rebuilt around it: the
/// mutation moves the player from the cell their feet are in to the one the break
/// opened, one block up, and the scenario reddens on that and on nothing else — the
/// break report and the refusal are identical either way, which is what says the
/// position is carrying the signal.
///
/// **It reads where the run *seated* the player, and it used to read the position the
/// fixture declared.** Those were the same value until entry clearing shipped, and
/// then they stopped being: a player declared inside solid rock with somewhere clear
/// nearby is moved by the admission door before the reload under test ever happens.
/// A guard reading the declared spawn therefore went on passing while being false of
/// the run — the exact shape of a scan that can no longer look at the thing it
/// guards. Taking the published snapshot rather than a `Vec3` is what makes handing
/// it a declared constant unspellable.
///
/// # Errors
///
/// Returns an error naming which half failed and what it found, or saying that the
/// client has published nothing to read a position out of.
pub fn require_a_refusal_could_have_moved_them(
    blocks: &VoxelWorld,
    seated: Option<Arc<SimSnapshot>>,
) -> Result<(), Box<dyn Error>> {
    let feet = seated
        .ok_or(
            "this scenario's premise is about where the run seated the player, and the client has published no snapshot for one to be read out of",
        )?
        .player
        .position;
    let serving = overlap_at(blocks, &SOLID_WHILE_SERVING, feet);
    let once_water_is = overlap_at(blocks, &SOLID_ONCE_WATER_IS, feet);
    require(
        matches!(serving, Overlap::Overlapping(_)),
        format!(
            "this scenario's named mutation calls the clearing search from the refusal path, where the world still has the solidity it had — so the box where the run seated the player, at {feet:?}, has to overlap something solid there or that call has no work to do and the mutation cannot bite. It overlaps {serving:?}"
        ),
    )?;
    require(
        overlapping_count(&once_water_is) > overlapping_count(&serving),
        format!(
            "this scenario needs a refused candidate that would have made solid a *further* cell the player's box overlaps, and it overlaps {serving:?} while serving against {once_water_is:?} once `base:water` is solid"
        ),
    )?;
    let somewhere = clear_within_the_search(blocks, &SOLID_WHILE_SERVING, feet);
    require(
        !somewhere.is_empty(),
        format!(
            "this scenario's named mutation moves the player only where the search it wrongly reaches finds somewhere to put them, and nothing within {A_SEARCH_OF} blocks of {feet:?} is clear under the content serving — so that call would answer that it found nowhere, move nobody, and leave the scenario green"
        ),
    )
}

/// How many cells an overlap found.
fn overlapping_count(overlap: &Overlap) -> usize {
    match overlap {
        Overlap::Clear => 0,
        Overlap::Overlapping(cells) => cells.len(),
    }
}

/// Refuses unless every position the search may look at from `feet` is blocked once
/// water is solid.
///
/// **The premise of a scenario with nowhere to go, asserted rather than described.** A
/// world meant to have
/// nowhere clear inside the bound is a world of two and a half thousand cells, and a
/// gap left anywhere in it makes the scenario fail as though the search had gone
/// looking where it should not have. This walks the cube the search walks — cell
/// centres at `dx, dz ∈ [-8, 8]` and `dy ∈ [0, 8]` — over the oracle above, so a gap
/// is reported as the gap it is.
///
/// # Errors
///
/// Returns an error naming how many positions were clear and where the first of them
/// was.
pub fn require_nothing_clear_within_the_search(
    blocks: &VoxelWorld,
    feet: Vec3,
) -> Result<(), Box<dyn Error>> {
    let clear = clear_within_the_search(blocks, &SOLID_ONCE_WATER_IS, feet);
    require(
        clear.is_empty(),
        format!(
            "this scenario needs a world with nowhere clear inside the declared search, and \
             {count} of the positions it may look at are clear — the first of them at {first:?}",
            count = clear.len(),
            first = clear.first()
        ),
    )
}

/// Every position the search may look at from `feet` that `solid` leaves clear.
///
/// The one walk of the cube, read by the guard that needs it to be empty and by the
/// one that needs it not to be — which is what keeps "where the search may look"
/// stated once rather than once per direction.
fn clear_within_the_search(blocks: &VoxelWorld, solid: &[&str], feet: Vec3) -> Vec<Vec3> {
    every_candidate()
        .map(|offset| candidate_feet(feet, offset))
        .filter(|position| overlap_at(blocks, solid, *position) == Overlap::Clear)
        .collect()
}

/// Every offset the search may look at.
///
/// The cube `dx, dz ∈ [-8, 8]` with `dy ∈ [0, 8]`: downward is absent from it rather
/// than ranked last, which is what makes "never downward" a property of the candidate
/// set.
fn every_candidate() -> impl Iterator<Item = Cell> {
    let reach = A_SEARCH_OF as i32;
    (0..=reach).flat_map(move |dy| {
        (-reach..=reach).flat_map(move |dz| (-reach..=reach).map(move |dx| (dx, dy, dz)))
    })
}

/// Where the search would put a player whose feet are at `feet` for the cell `offset`
/// away from the one they are in — the centre of that cell, feet on its floor.
fn candidate_feet(feet: Vec3, offset: Cell) -> Vec3 {
    let (across, up, along) = offset;
    let (x, y, z) = floor_cell(feet);
    Vec3::new(
        (x + across) as f32 + 0.5,
        (y + up) as f32,
        (z + along) as f32 + 0.5,
    )
}

/// Refuses unless the player is moving upward.
///
/// The premise of the scenario about what a cleared player's velocity becomes. Every
/// destination the search reaches upward is a cell whose
/// own floor is the thing that blocked the candidate one step lower, so a cleared
/// player always lands supported — and a *downward* velocity is then spent by the very
/// next tick's collision whether the search zeroed it or not. Only a rise survives
/// that tick, so only a rise can carry the signal.
///
/// # Errors
///
/// Returns an error where the player is not rising.
pub fn require_rising(spawn: &PlayerState) -> Result<(), Box<dyn Error>> {
    require(
        spawn.velocity.y > 0.0,
        format!(
            "this scenario is about the velocity a clearing move takes away, and a downward one is \
             taken away by the next tick's collision regardless — so the player has to be rising \
             when the swap happens, and this one is moving at {velocity:?}",
            velocity = spawn.velocity
        ),
    )
}

/// A player standing still at `feet`, with nothing under them and about to fall.
#[must_use]
pub const fn airborne_at(feet: Vec3) -> PlayerState {
    rising_at(feet, 0.0)
}

/// A player at `feet` moving upward at `speed` blocks per second.
#[must_use]
pub const fn rising_at(feet: Vec3, speed: f32) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::new(0.0, speed, 0.0),
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// The shape of a world a clearing scenario is driven over.
///
/// A group rather than five arguments — which the constitution's four-argument limit
/// forces and which reads better anyway, since what a scenario states about its world
/// is one thing.
#[derive(Debug)]
pub struct Shape<'a> {
    /// How many chunk columns square the world is.
    pub columns: u32,
    /// The block the one solid layer at [`FLOOR_ROW`] is made of, or nothing where the
    /// world carries no floor at all.
    pub floor: Option<&'a str>,
    /// The columns that layer is missing from, so that the cell one block below a
    /// player can be a clear position.
    pub open: &'a [(u32, u32)],
    /// Cells written above the floor, each naming its own block.
    pub cells: &'a [(Cell, &'a str)],
}

/// The world `shape` declares, named against `registry`.
///
/// # Errors
///
/// Returns an error if a name does not parse, if a cell lies outside the world, or if
/// the world refuses a write.
pub fn a_world(registry: &BlockRegistry, shape: &Shape<'_>) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(shape.columns);
    lay_the_floor(&mut blocks, registry, shape)?;
    for (cell, held) in shape.cells {
        blocks.set_block(inside(*cell)?, &BlockName::parse(held)?, registry)?;
    }
    Ok(blocks)
}

/// Writes `shape`'s floor into `blocks`, wherever it is not open.
fn lay_the_floor(
    blocks: &mut VoxelWorld,
    registry: &BlockRegistry,
    shape: &Shape<'_>,
) -> Result<(), Box<dyn Error>> {
    let Some(floor) = shape.floor else {
        return Ok(());
    };
    let named = BlockName::parse(floor)?;
    let layer = u32::try_from(FLOOR_ROW)?;
    let across = shape.columns * SECTION_SIZE;
    for (x, z) in every_position(across).filter(|at| !shape.open.contains(at)) {
        blocks.set_block(WorldPos { x, y: layer, z }, &named, registry)?;
    }
    Ok(())
}

/// Every horizontal position of a world `across` blocks square.
fn every_position(across: u32) -> impl Iterator<Item = (u32, u32)> {
    (0..across).flat_map(move |z| (0..across).map(move |x| (x, z)))
}

/// Every cell within the declared search of `centre` horizontally, at each of `rows`.
///
/// The reach is [`A_SEARCH_OF`], so a fixture that has to fill everything the search
/// can look at follows the declared bound rather than a number of its own — and
/// follows it if it ever changes.
#[must_use]
pub fn within_the_search(centre: Cell, rows: &[i32]) -> Vec<Cell> {
    let (centre_x, _, centre_z) = centre;
    let reach = A_SEARCH_OF as i32;
    let mut cells = Vec::new();
    for row in rows {
        cells.extend(square(centre_x, centre_z, reach).map(|(x, z)| (x, *row, z)));
    }
    cells
}

/// Every horizontal position within `reach` of a centre.
fn square(centre_x: i32, centre_z: i32, reach: i32) -> impl Iterator<Item = (i32, i32)> {
    (centre_z - reach..=centre_z + reach)
        .flat_map(move |z| (centre_x - reach..=centre_x + reach).map(move |x| (x, z)))
}

/// Each of `cells` holding `block`, as a [`Shape`]'s cell list.
#[must_use]
pub fn each_holding<'a>(cells: &[Cell], block: &'a str) -> Vec<(Cell, &'a str)> {
    cells.iter().map(|cell| (*cell, block)).collect()
}

/// A client at `spawn` playing the world `blocks_of` builds over the root at `root`,
/// and a copy of that world for [`overlap_at`] to read.
///
/// **The copy is what makes the guards above possible at all.** `Session` hands out no
/// borrow of the world it owns — deliberately — so a fixture that wants to ask where
/// the solid cells are has to keep the world it declared.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or if the
/// content declares no solid block at all.
pub fn a_client_over(
    root: &Path,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<(InputHarness, VoxelWorld), Box<dyn Error>> {
    let registry = registry_of(root)?;
    let blocks = blocks_of(&registry)?;
    let declared = blocks.clone();
    let (simulation, holding) = playing(root, spawn, move |_| Ok(blocks))?;
    let mut client = InputHarness::started();
    client.play(simulation, holding);
    Ok((client, declared))
}

/// The same, with the client also watching the root it plays through a double.
///
/// **The root it plays and the root it watches are one directory**, which is the
/// arrangement a mod author is in: the file they edit is the file the run was started
/// from.
///
/// # Errors
///
/// Returns whatever [`a_client_over`] refuses.
pub fn a_client_watching(
    root: &ContentRoot,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<(InputHarness, VoxelWorld, Reports), Box<dyn Error>> {
    let (mut client, declared) = a_client_over(root.path(), spawn, blocks_of)?;
    let (watching, reports) = watch();
    client.attach_reload(ContentReload::watching(
        root.path().to_owned(),
        Box::new(watching),
    ));
    Ok((client, declared, reports))
}
