//! The worlds a reload is driven over, where the player stands in them, where
//! they are looking, and what one action in them reported.
//!
//! # Every world here is named against a real content root
//!
//! The registry a world is built from is the one [`mc_sim::content::load`] hands
//! back for a content root on disk, never one assembled in Rust. A candidate is
//! another read of another root, so the two have to be the same *kind* of thing
//! or the swap under test would be between a fixture's invention and the
//! product's own content.
//!
//! # The aims are derived, and both of them are stated here once
//!
//! Look is `0.0022` radians per raw device count
//! (`crates/mc-sim/src/player/input.rs`), the eye stands `1.62` blocks over the
//! feet, and yaw zero looks along `+x`. From the spawn below, at feet
//! `(8.5, 10.0, 8.5)`:
//!
//! - `280` counts down is `0.616` rad, `35.29°`. The ray meets the floor's top
//!   face at `1.62 / tan 35.29° = 2.288` blocks out, at `x = 10.788` — inside
//!   voxel column 10, entered through its `+Y` face. That is [`THE_FAR_CELL`],
//!   and a placement against it lands in [`OVER_THE_FAR_CELL`].
//! - a further `196` counts is `476` in all: `1.0472` rad, `60.0°`. The same face
//!   is met `1.62 / tan 60° = 0.935` blocks out, at `x = 9.435` — voxel column 9.
//!   That is [`THE_NEAR_CELL`], and a placement lands in [`OVER_THE_NEAR_CELL`].
//!
//! Neither cell lies in the player's own box, which spans `x ∈ [8.2, 8.8]`, so a
//! placement into either is never refused for standing in it. Both are well
//! inside the reach of 5.0 blocks measured from the eye: the further is 2.80
//! blocks away and the nearer 1.87.
//!
//! **Two aims and not one, because a single aim cannot break one cell and place
//! in another.** A break empties the cell the ray stopped at, and the next
//! placement along that same ray lands in exactly the cell just emptied — so a
//! scenario about a broken cell *and* a placed cell would be about one cell
//! twice.
//!
//! # A save is how a world is read after the swap, and that half lives next door
//!
//! `Session` hands out no borrow of what it owns — no accessor for the
//! simulation and none for the world — which is a property of that type rather
//! than an oversight. So a scenario about what the world still holds reads it the
//! way a player would: by quitting, and by loading what was written.
//!
//! **All of that is [`super::reload_save`]**, which was part of this file until it
//! reached the test-file size limit. The seam is the one that limit exists to
//! force: this module is what a reload is *driven over* — the world, where the
//! player stands in it, where they are looking, and what one action in it reported
//! — and the sibling is what is *read back afterwards*. Two cells and one
//! conversion cross the line, and they are `pub` here for that reason alone.
//!
//! # Why this is reached by `#[path]` and not declared inside `support`
//!
//! It names types the implementation has not written yet, and a module declared
//! in `support/mod.rs` is compiled into every binary that says `mod support;` —
//! which would leave the whole crate's tests unable to build for the whole of the
//! window before the swap lands. A binary including this must declare
//! `mod support;` as well.

// Each scenario binary links this whole module and drives a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use glam::Vec3;
use mc_core::block::{BlockRegistry, RegistryError};
use mc_core::content::LayerAssignment;
use mc_core::id::BlockName;
use mc_sim::action::{EditReport, Refusal, default_held_block};
use mc_sim::player::{BlockPos, PlayerState};
use mc_sim::simulation::{PublishedContent, SimSnapshot, Simulation, seat};
use mc_sim::world::World;
use mc_world::column::{COLUMN_HEIGHT, SECTIONS_PER_COLUMN};
use mc_world::section::{Contents, SECTION_SIZE};
use mc_world::world::{VoxelWorld, WorldError, WorldPos};

/// A cell, spelled the way a player's report spells one so that a fixture's
/// expectation and the client's answer are the same kind of value.
pub type Cell = (i32, i32, i32);

/// How many chunk columns across the small worlds here are.
pub const COLUMNS: u32 = 1;

/// How many blocks across one column is, and how tall.
pub const ACROSS: u32 = SECTION_SIZE;
pub const HEIGHT: u32 = COLUMN_HEIGHT;

/// The one solid layer of the small worlds' floor. Feet come to rest on its top
/// face, one above it.
pub const FLOOR: i32 = 9;

/// Where the player stands: on the floor, centred in the column, facing along
/// `+x`, holding still.
pub const SPAWN: Vec3 = Vec3::new(8.5, 10.0, 8.5);

/// How many cells one of the small worlds declares.
///
/// Derived from the declaration rather than counted from a comparison, so a walk
/// that visited a smaller world fails loudly instead of agreeing over fewer
/// cells.
pub const EVERY_CELL_OF_ONE_COLUMN: usize = (COLUMNS * ACROSS * COLUMNS * ACROSS * HEIGHT) as usize;

/// How many cells the shipped world declares — four columns square, sixteen
/// sections per column.
pub const EVERY_CELL_OF_THE_SHIPPED_WORLD: usize = (mc_sim::replay::world::FOOTPRINT_COLUMNS
    * SECTION_SIZE
    * mc_sim::replay::world::FOOTPRINT_COLUMNS
    * SECTION_SIZE
    * SECTIONS_PER_COLUMN
    * SECTION_SIZE) as usize;

/// Raw device counts that aim the spawn's look at [`THE_FAR_CELL`], and the
/// further counts that carry it on to [`THE_NEAR_CELL`].
pub const AIM_AT_THE_FAR_CELL: f64 = 280.0;
pub const AIM_ON_TO_THE_NEAR_CELL: f64 = 196.0;

/// The two floor cells those aims meet, and the cells a placement against each
/// of them lands in.
pub const THE_FAR_CELL: Cell = (10, FLOOR, 8);
pub const OVER_THE_FAR_CELL: Cell = (10, FLOOR + 1, 8);
pub const THE_NEAR_CELL: Cell = (9, FLOOR, 8);
pub const OVER_THE_NEAR_CELL: Cell = (9, FLOOR + 1, 8);

/// The floor cell the player's own box stands on.
pub const UNDER_THE_SPAWN: Cell = (8, FLOOR, 8);

/// The layer a ceiling sits in, three blocks over the floor.
///
/// The player's box reaches `y = 11.8` and the ceiling's underside is at `13.0`,
/// so it clears their head by 1.2 blocks and no walk can touch it. Their eye
/// stands at `11.62`, which puts the ceiling 1.38 blocks straight up — well
/// inside the reach of 5.0 blocks a look is bounded by.
pub const CEILING: i32 = 13;

/// Raw device counts that aim the spawn's look at the ceiling.
///
/// `706` counts is `1.5532` rad, `88.99°` — just under the declared pitch limit
/// of 89°, so nothing is clamped and the angle is the one this asks for. At that
/// pitch the ray drifts `1.38 / tan 88.99° = 0.024` blocks sideways on its way
/// up, so it meets the ceiling over whichever column the player is standing in.
///
/// **Negative, because raw vertical counts are subtracted from pitch**: a
/// positive count looks down and this looks up.
pub const AIM_AT_THE_CEILING: f64 = -706.0;

/// What a cell holding nothing is called wherever this module reports contents as
/// text.
///
/// Not a block name and unable to become one: every namespaced name carries a
/// colon, so an expectation of an empty cell and one of a named block can sit
/// side by side without either impersonating the other.
pub const NOTHING: &str = "nothing";

/// Every block the content root at `root` declares.
///
/// # Errors
///
/// Returns whichever reader refused the root.
pub fn registry_of(root: &Path) -> Result<Arc<BlockRegistry>, Box<dyn Error>> {
    Ok(Arc::new(
        mc_sim::content::load(root, &LayerAssignment::none())?.registry,
    ))
}

/// One column whose only solid layer is a floor of `block`, and nothing else.
///
/// # Errors
///
/// Returns an error if the name does not parse or the world refuses a write.
pub fn floor_of(registry: &BlockRegistry, block: &str) -> Result<VoxelWorld, Box<dyn Error>> {
    floor_holding(registry, block, &[])
}

/// The same floor with further cells written into it, each naming its own block.
///
/// # Errors
///
/// Returns an error if a name does not parse, if a cell lies outside the world,
/// or if the world refuses a write.
pub fn floor_holding(
    registry: &BlockRegistry,
    block: &str,
    cells: &[(Cell, &str)],
) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = VoxelWorld::empty(COLUMNS);
    let floor = BlockName::parse(block)?;
    let layer = u32::try_from(FLOOR)?;
    for (x, z) in every_column() {
        blocks.set_block(WorldPos { x, y: layer, z }, &floor, registry)?;
    }
    for (at, held) in cells {
        blocks.set_block(inside(*at)?, &BlockName::parse(held)?, registry)?;
    }
    Ok(blocks)
}

/// One column with a floor of `floor` and, three blocks over it, a whole layer of
/// `ceiling`.
///
/// **The two are different blocks on purpose, and which one is which is the whole
/// of it.** A scenario asking whether a swap left the player alone cannot have the
/// player standing on the block whose solidity the swap takes away — the next tick
/// would then drop them *legitimately*, and the comparison would fail against a
/// correct client. So the floor holds them up and the ceiling is what the changed
/// block is, reachable by a look and by nothing else the player does.
///
/// A whole layer rather than one block, so that a player who has walked or turned
/// still meets it looking up. Nothing here depends on where they ended up.
///
/// # Errors
///
/// Returns an error if a name does not parse or the world refuses a write.
pub fn floor_under_a_ceiling(
    registry: &BlockRegistry,
    floor: &str,
    ceiling: &str,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let mut blocks = floor_of(registry, floor)?;
    let overhead = BlockName::parse(ceiling)?;
    let layer = u32::try_from(CEILING)?;
    for (x, z) in every_column() {
        blocks.set_block(WorldPos { x, y: layer, z }, &overhead, registry)?;
    }
    Ok(blocks)
}

/// The world the client launches into, regenerated from its declared seed.
///
/// # Errors
///
/// Returns an error if the world cannot be generated out of what `registry`
/// knows.
pub fn shipped_world(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    Ok(
        mc_sim::replay::ReplayWorld::generate(mc_sim::REPLAY_SEED, registry)?
            .blocks()
            .clone(),
    )
}

/// A simulation of the world `blocks_of` builds against the root at `root`, with
/// the player at `spawn`, and the block a place request over it names.
///
/// The held block is asked of the simulation's own policy rather than spelled
/// here, so every fixture drives the client through the decision the composition
/// root makes.
///
/// # Errors
///
/// Returns an error if the root does not read, if the world does not build, or
/// if the content declares no solid block at all.
pub fn playing(
    root: &Path,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
) -> Result<(Simulation, BlockName), Box<dyn Error>> {
    playing_serving(root, spawn, blocks_of, &LayerAssignment::none())
}

/// The same, for a session that has already spent `spent` layers.
///
/// **A launch passes [`LayerAssignment::none`] and that is a fact rather than a
/// decision**, which is why [`playing`] states it at the call. A scenario about
/// what happens near the end of the session's budget cannot reach that state by
/// launching, because reaching it organically takes hundreds of reloads — so the
/// layers already spent are handed over here, and the content root is read
/// against them exactly as a reload's build stage reads one.
///
/// # Errors
///
/// Returns an error if the root does not read against those layers, if the world
/// does not build, or if the content declares no solid block at all.
pub fn playing_serving(
    root: &Path,
    spawn: PlayerState,
    blocks_of: impl FnOnce(&BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>>,
    spent: &LayerAssignment,
) -> Result<(Simulation, BlockName), Box<dyn Error>> {
    let loaded = mc_sim::content::load(root, spent)?;
    let registry = Arc::new(loaded.registry);
    let blocks = blocks_of(&registry)?;
    let holding = default_held_block(&registry)
        .ok_or("this fixture's content root declares no solid block for a client to hold")?;
    let content = PublishedContent::first(loaded.resolved, loaded.hud);
    Ok((
        seat(spawn, World::new(blocks, Arc::clone(&registry))?, content).simulation,
        holding,
    ))
}

/// A player standing still on the floor, looking level along `+x`.
#[must_use]
pub const fn standing() -> PlayerState {
    standing_at(SPAWN)
}

/// A player standing still at `feet`, looking level along `+x`.
#[must_use]
pub const fn standing_at(feet: Vec3) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    }
}

/// A player in the air over the spawn column at `height`, at rest and about to
/// fall.
#[must_use]
pub const fn falling_from(height: f32) -> PlayerState {
    PlayerState {
        position: Vec3::new(SPAWN.x, height, SPAWN.z),
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    }
}

/// Where the player published in `snapshot` stands and which way it faces, as
/// the integers those floats are.
///
/// **Compared as bits and never with a tolerance.** What these scenarios ask is
/// whether a swap moved the player *at all*, and any tolerance answers "not much"
/// to a question that is about "not".
#[must_use]
pub fn standing_and_facing(snapshot: &SimSnapshot) -> ([u32; 3], u32, u32) {
    (
        snapshot.player.position.to_array().map(f32::to_bits),
        snapshot.player.yaw.to_bits(),
        snapshot.player.pitch.to_bits(),
    )
}

/// How fast the player published in `snapshot` is moving, as the integers those
/// floats are.
#[must_use]
pub fn moving_at(snapshot: &SimSnapshot) -> [u32; 3] {
    snapshot.player.velocity.to_array().map(f32::to_bits)
}

/// A velocity of nothing at all, for a fixture guard that has to know the player
/// was moving before a swap is asked to leave that alone.
#[must_use]
pub fn at_rest() -> [u32; 3] {
    [0.0_f32.to_bits(); 3]
}

/// How high the player published in `snapshot` stands, and whether the world is
/// holding them up.
#[must_use]
pub const fn resting(snapshot: &SimSnapshot) -> (f32, bool) {
    (snapshot.player.position.y, snapshot.player.on_ground)
}

/// Which tick `snapshot` was published under.
#[must_use]
pub const fn published_tick(snapshot: &SimSnapshot) -> u32 {
    snapshot.tick
}

/// What one requested action did, as a value a scenario can compare whole.
///
/// **Enumerated rather than the report itself**, because a refusal the store
/// raised carries an error chain a scenario cannot spell — and because "the cell
/// was emptied" and "the cell was left holding something" are the two answers
/// these scenarios are about, told apart by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// The cell at this position was emptied.
    Emptied(Cell),
    /// The cell at this position now holds this block, having held that one.
    Wrote {
        cell: Cell,
        from: String,
        to: String,
    },
    /// Content declares the block cannot be broken.
    Indestructible,
    /// The cell holds a block content does not declare replaceable.
    Occupied,
    /// Nothing solid lies along the look direction inside the reach.
    NoTarget,
    /// The action named a block the content now serving does not declare, and
    /// this is the name it wanted.
    ///
    /// The residue a break leaves behind is resolved when the break happens and
    /// not when the declaration is read, so this is where a `breaks_into` naming
    /// nothing arrives — which is the whole of what the late-resolution contract
    /// costs.
    NamedABlockNothingDeclares(String),
    /// The action was refused for some other reason, rendered.
    RefusedOtherwise(String),
    /// The tick carried no action at all.
    NothingWasAsked,
}

/// What one tick's report says, as an [`Edit`].
#[must_use]
pub fn edit(report: Option<EditReport>) -> Edit {
    match report {
        None => Edit::NothingWasAsked,
        Some(EditReport::Changed { cell, from, to }) => changed(cell, &from, &to),
        Some(EditReport::Refused(Refusal::Indestructible)) => Edit::Indestructible,
        Some(EditReport::Refused(Refusal::Occupied)) => Edit::Occupied,
        Some(EditReport::Refused(Refusal::NoTarget)) => Edit::NoTarget,
        Some(EditReport::Refused(Refusal::Storage(WorldError::Registry(
            RegistryError::UnknownName { name },
        )))) => Edit::NamedABlockNothingDeclares(name.as_str().to_owned()),
        Some(EditReport::Refused(other)) => Edit::RefusedOtherwise(format!("{other:?}")),
    }
}

/// An action refused because it named `block`, which no declaration declares.
#[must_use]
pub fn named_nothing_declared(block: &str) -> Edit {
    Edit::NamedABlockNothingDeclares(block.to_owned())
}

/// An edit that wrote `to` over `from` at `cell`, for a scenario to compare
/// against.
#[must_use]
pub fn wrote(cell: Cell, from: &str, to: &str) -> Edit {
    Edit::Wrote {
        cell,
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

/// One changed cell, as an [`Edit`].
fn changed(cell: BlockPos, from: &Contents, to: &Contents) -> Edit {
    let at = (cell.x, cell.y, cell.z);
    match to {
        Contents::Empty => Edit::Emptied(at),
        Contents::Holds(_) => Edit::Wrote {
            cell: at,
            from: described_contents(from.as_ref()),
            to: described_contents(to.as_ref()),
        },
    }
}

/// A cell as the world spells a position.
///
/// # Errors
///
/// Returns an error where the cell lies on the negative side of an axis, which a
/// world has no position for.
pub fn inside(at: Cell) -> Result<WorldPos, Box<dyn Error>> {
    let (x, y, z) = at;
    Ok(WorldPos {
        x: u32::try_from(x)?,
        y: u32::try_from(y)?,
        z: u32::try_from(z)?,
    })
}

/// Every horizontal position of one column.
fn every_column() -> impl Iterator<Item = (u32, u32)> {
    (0..ACROSS).flat_map(|z| (0..ACROSS).map(move |x| (x, z)))
}

/// What `contents` holds, as text: the block's own name, or [`NOTHING`].
///
/// Two arms rather than a fallback, because "this cell holds nothing" and "this
/// cell holds a block" are different facts and a default would let one arrive
/// under the other's name. `pub` because [`super::reload_save`] reads a cell out
/// of a save and has to spell it the same way an edit here does.
pub fn described_contents(contents: Contents<&BlockName>) -> String {
    match contents {
        Contents::Empty => NOTHING.to_owned(),
        Contents::Holds(name) => name.as_str().to_owned(),
    }
}
