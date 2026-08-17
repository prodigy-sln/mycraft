//! What the simulation does with a candidate that stops declaring a block.
//!
//! # The world holds names, and only a name some cell still holds counts
//!
//! A section's palette carries an entry for every block that was ever written
//! into it, whether or not any cell still refers to one. Reading the palette
//! would therefore refuse a reload over a block the player broke ten minutes ago
//! — a defect and not a cost, and the reason the fixture below reaches its
//! no-water world by **taking the water out of a world that held it** rather than
//! by building one that never had any.
//!
//! **That constraint is what these scenarios are worth, so it is asserted rather
//! than described.** A world that never held water leaves the two readings
//! indistinguishable, and every answer below would be satisfied by either. The
//! fixture guard states what the section actually is: it names water, and no cell
//! of it holds any.
//!
//! # The order of a refusal is a claim, not a convenience
//!
//! An author who renamed two blocks has to see both, somewhere they can look
//! things up. The fixture puts stone in a **lower** section than grass, so a
//! refusal listing whichever it came across first would name stone first — which
//! is exactly what an ascending order forbids.
//!
//! # Why these drive the simulation rather than a client
//!
//! What is asked here is the simulation's own answer: which candidates it admits,
//! and what it says when it turns one away. Nothing about it reaches a player
//! except through a refusal a later phase prints, and every scenario asking
//! whether the *client* honours an admitted candidate lives in
//! `crates/mc-client/tests/`.

#[path = "support/roots.rs"]
mod roots;
mod support;

use std::error::Error;
use std::ops::Range;
use std::sync::Arc;

use glam::Vec3;
use mc_core::id::BlockName;
use mc_sim::player::PlayerState;
use mc_sim::simulation::Simulation;
use mc_sim::world::World;
use mc_world::section::{Contents, SECTION_SIZE};
use mc_world::world::{VoxelWorld, WorldPos};

use roots::{
    GRASS_FILE, STONE_FILE, WATER_FILE, accepted, adoption, holding_blocks_it_does_not_declare,
    shipped,
};
use support::{DIRT, GRASS, STONE, TestResult, WATER, content_registry, published_content};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// Where the fixture writes the block it wants held low in the world, and the one
/// it wants held high.
///
/// **The two sit in different sections and the low one is written first**, which
/// is what makes "whichever was found first" and "ascending" two different
/// answers rather than the same one.
const LOW: WorldPos = WorldPos { x: 1, y: 9, z: 1 };
const HIGH: WorldPos = WorldPos { x: 1, y: 20, z: 1 };

/// Which section of the fixture's one column each of those two lands in.
const LOW_SECTION: usize = 0;
const HIGH_SECTION: usize = 1;

/// Where the player stands. Nothing here is about the player, and the height is
/// well clear of everything the fixture writes.
const ABOVE_EVERYTHING: Vec3 = Vec3::new(8.5, 40.0, 8.5);

#[test]
fn a_candidate_that_stops_declaring_a_block_the_world_holds_is_refused_naming_it() -> TestResult {
    let mut simulation = playing(written(&[(LOW, STONE)], &[])?)?;
    let candidate = shipped()?.not_declaring(&[STONE_FILE])?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));

    assert_eq!(
        (answered, declares(&simulation, STONE)),
        (holding_blocks_it_does_not_declare(&[STONE]), true),
        "the world holds stone and the candidate declares none, so nothing could go in the cells \
         holding it — and that is not a judgement to make on the author's behalf. The refusal has \
         to name the block they have to put back, and the content that was serving has to go on \
         serving: a swap that half-applied would leave a world named against a registry that has \
         stopped knowing what is in it"
    );
    Ok(())
}

#[test]
fn a_candidate_dropping_a_block_no_cell_holds_any_more_is_accepted() -> TestResult {
    let blocks = written(&[(LOW, STONE), (HIGH, WATER)], &[HIGH])?;
    require_named_but_unheld(&blocks, HIGH_SECTION, WATER)?;
    let mut simulation = playing(blocks)?;
    let candidate = shipped()?.not_declaring(&[WATER_FILE])?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));

    assert_eq!(
        (answered, declares(&simulation, WATER)),
        (accepted(DIRT), false),
        "the world held water and the player took it out, so no cell holds any and the author is \
         free to stop declaring it. A check reading the section's palette instead of what its \
         cells still refer to refuses this — turning a block somebody broke ten minutes ago into a \
         permanent obligation on every candidate after it"
    );
    Ok(())
}

#[test]
fn a_refusal_names_every_block_the_world_holds_that_the_candidate_dropped_ascending() -> TestResult
{
    let blocks = written(&[(LOW, STONE), (HIGH, GRASS)], &[])?;
    require_held(&blocks, LOW_SECTION, STONE)?;
    require_held(&blocks, HIGH_SECTION, GRASS)?;
    let mut simulation = playing(blocks)?;
    let candidate = shipped()?.not_declaring(&[GRASS_FILE, STONE_FILE])?;

    let answered = adoption(mc_sim::reload::adopt_at_tick_boundary(
        &mut simulation,
        candidate.candidate()?,
    ));

    assert_eq!(
        answered,
        holding_blocks_it_does_not_declare(&[GRASS, STONE]),
        "both blocks are gone and the author has to see both, in an order they can look things up \
         in. This world holds stone lower than grass, so a refusal reporting whichever it came \
         across first names stone first and reads as a list nobody sorted — and one that reported \
         a single block would cost an author one save per renamed block"
    );
    Ok(())
}

/// A simulation of `blocks`, with the player well clear of everything in it.
fn playing(blocks: VoxelWorld) -> Result<Simulation, Box<dyn Error>> {
    let registry = Arc::new(content_registry()?);
    let spawn = PlayerState {
        position: ABOVE_EVERYTHING,
        velocity: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        on_ground: false,
    };
    let content = published_content(&registry)?;
    Ok(Simulation::new(
        spawn,
        World::new(blocks, registry)?,
        content,
    ))
}

/// One empty column with `cells` written into it, and then `emptied` taken back
/// out of it.
///
/// **Taking a cell back out is the whole point of the second list.** The write
/// path maintains the reference counts a palette entry survives on, so a cell
/// written and then emptied leaves the section naming a block no cell holds —
/// the state a player who broke a block leaves behind, and the only state in
/// which the two readings of "what does this world hold" disagree.
fn written(cells: &[(WorldPos, &str)], emptied: &[WorldPos]) -> Result<VoxelWorld, Box<dyn Error>> {
    let registry = content_registry()?;
    let mut blocks = VoxelWorld::empty(COLUMNS);
    for (at, block) in cells {
        blocks.set_block(*at, &BlockName::parse(block)?, &registry)?;
    }
    for at in emptied {
        blocks.empty_at(*at)?;
    }
    Ok(blocks)
}

/// Refuses unless the section at `index` names `block` while no cell of it holds
/// one.
///
/// The fixture constraint this file rests on, asserted rather than described:
/// without it the scenario passes over a world that never had water in it, and
/// stops being about anything.
fn require_named_but_unheld(
    blocks: &VoxelWorld,
    index: usize,
    block: &str,
) -> Result<(), Box<dyn Error>> {
    if !names(blocks, index, block)? {
        return Err(format!(
            "this fixture has to leave section {index} naming {block} after the cells holding it \
             were emptied, and it does not. What it built is a world that never held the block \
             rather than one a player took it out of, and the scenario passes over either"
        )
        .into());
    }
    if holds(blocks, index, block) {
        return Err(format!(
            "this fixture has to leave no cell of section {index} holding {block}, and one still \
             does. The candidate would then be refused over a block the world really holds, which \
             is a different scenario"
        )
        .into());
    }
    Ok(())
}

/// Refuses unless some cell of the section at `index` holds `block`.
fn require_held(blocks: &VoxelWorld, index: usize, block: &str) -> Result<(), Box<dyn Error>> {
    if holds(blocks, index, block) {
        return Ok(());
    }
    Err(format!(
        "this fixture has to leave section {index} holding {block}, and it does not — so the order \
         a refusal reports its blocks in would be an order over one name"
    )
    .into())
}

/// Whether the palette of the section at `index` names `block`, whether or not
/// any cell still holds one.
fn names(blocks: &VoxelWorld, index: usize, block: &str) -> Result<bool, Box<dyn Error>> {
    let section = blocks
        .section_at(mc_world::column::ColumnCoordinate { x: 0, z: 0 }, index)
        .ok_or_else(|| format!("this fixture's world has no section {index} to read"))?;
    Ok(section
        .palette()
        .any(|entry| matches!(entry, Contents::Holds(name) if name.as_str() == block)))
}

/// Whether any cell of the section at `index` holds `block`.
fn holds(blocks: &VoxelWorld, index: usize, block: &str) -> bool {
    let Some(rows) = rows_of(index) else {
        return false;
    };
    blocks
        .extent()
        .positions()
        .filter(|at| rows.contains(&at.y))
        .any(
            |at| matches!(blocks.block_at(at), Ok(Contents::Holds(held)) if held.as_str() == block),
        )
}

/// Which rows of the world the section at `index` covers.
fn rows_of(index: usize) -> Option<Range<u32>> {
    let low = u32::try_from(index).ok()?.checked_mul(SECTION_SIZE)?;
    Some(low..low.checked_add(SECTION_SIZE)?)
}

/// Whether the registry the simulation's world is named against still declares
/// `block`.
fn declares(simulation: &Simulation, block: &str) -> bool {
    BlockName::parse(block).is_ok_and(|name| simulation.world().registry().resolve(&name).is_ok())
}
