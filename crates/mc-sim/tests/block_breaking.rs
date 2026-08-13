//! What a break leaves behind, and what content's silence about a block means.
//!
//! Content makes two independent claims and this file is where the difference
//! is read. `breakable` says whether the block can be broken at all; `breaks_into`
//! says what the cell then holds, and naming nothing means the cell is *emptied*
//! rather than that the block is indestructible. Collapsing the two — reading a
//! missing residue as "cannot be broken", which is the shape this feature first
//! shipped — makes every ordinary block in the game indestructible, and the third
//! scenario below is what says so.
//!
//! # Why the fixture registers its own blocks
//!
//! MVP 1's base game deliberately ships no indestructible block. That is a scope
//! choice and not a fact about how breakability is encoded — any block may now
//! declare `breakable = false` — so a scenario about a block that cannot be
//! broken has to bring its own. Reaching for a shipped block instead would mean
//! reaching for one that is not solid and can therefore never be *targeted*, and
//! the scenario would go green because nothing was targeted rather than because
//! breakability was respected.
//!
//! The fixture carries a second block for the opposite reason. Breaking any
//! shipped block empties its cell, so a break that emptied the cell
//! unconditionally would satisfy "the block its own definition names" for all of
//! them. The fixture's breakable block names *dirt*, which nothing else in the
//! fixture would produce.
//!
//! # Emptiness is the world's word, and the fixture is built so that shows
//!
//! What a cell holds when it holds nothing is a fact about the world that was
//! built, not one Rust may pick — the engine may name no block at all
//! (`crates/mc-world/tests/no_hardcoded_block_names.rs`). The emptying scenario's
//! world is therefore filled with **water** rather than air, so that an
//! implementation naming air outright, or falling back to the first block the
//! registry registered, leaves the wrong block and is caught. Against an
//! air-filled fixture all three of those agree, and the assertion would be
//! reporting a coincidence.
//!
//! # The registration order is a fixture constraint, and it is asserted
//!
//! "The first registered block" is id 0, and it is only the base air block while
//! base content applies before the `fixture:` overlay. A registry built from the
//! overlay alone would number one of the fixture's own blocks 0, and the
//! water-filled scenario above would stop discriminating the fallback it exists
//! to discriminate. The last test here is what stops that being silent.

mod support;

use std::error::Error;

use glam::Vec3;
use mc_core::block::BlockId;
use mc_core::id::BlockName;
use mc_sim::action::{ActionIntent, TickIntent};
use mc_sim::player::{MovementIntent, PlayerState};
use mc_sim::simulation::Simulation;
use mc_world::world::WorldPos;

use support::chamber::{BlockChamber, CRUMBLING, UNBREAKABLE, at, differences, fixture_registry};
use support::{AIR, DIRT, STONE, TestResult, WATER};

/// How many chunk columns the fixture world spans on each axis.
const COLUMNS: u32 = 1;

/// The layer the floor occupies, so its top face is at `FLOOR_LAYER + 1`.
const FLOOR_LAYER: u32 = 9;

/// Where the feet stand: on the floor's top face.
const FEET_Y: f32 = 10.0;

/// The voxel row the eye is in, `floor(10.0 + 1.62)`.
const EYE_ROW: u32 = 11;

/// The block whose definition names dirt as what it breaks into.
///
/// Its near face is at x = 11.0 and the eye at x = 8.5, so it is met at 2.5
/// blocks — well inside the reach and with three cells of air in front of it.
const NAMES_ITS_RESIDUE: WorldPos = at(11, EYE_ROW, 8);

/// Where the feet stand for the residue scenario.
const FACING_IT: Vec3 = Vec3::new(8.5, FEET_Y, 8.5);

/// The block whose definition declares it cannot be broken, and the breakable
/// one beside it.
///
/// Both stand with their near face at x = 12.0, each on its own row of z, so the
/// two runs below differ in the feet's z and in nothing else — the same request,
/// from the same distance of 3.5 blocks, over one declared fixture.
const CANNOT_BE_BROKEN: WorldPos = at(12, EYE_ROW, 8);
const BESIDE_IT: WorldPos = at(12, EYE_ROW, 6);

/// The ordinary block whose definition names no residue, so breaking it empties
/// its cell. It stands exactly where the one that names dirt stands in its own
/// fixture: the two scenarios differ in what is declared there and in nothing
/// else.
const EMPTIED: WorldPos = NAMES_ITS_RESIDUE;

/// Where the feet stand to face each of the two.
const FACING_THE_INDESTRUCTIBLE: Vec3 = Vec3::new(8.5, FEET_Y, 8.5);
const FACING_THE_BREAKABLE: Vec3 = Vec3::new(8.5, FEET_Y, 6.5);

/// Yaw facing +x, which is where every ray in this file goes.
const ALONG_THE_ROW: f32 = 0.0;

#[test]
fn a_break_leaves_the_cell_holding_the_block_the_broken_ones_definition_names() -> TestResult {
    let chamber = one_block_that_names_its_residue();
    let declared = chamber.build()?;
    let broken = after_a_break(&chamber, standing(FACING_IT))?;

    assert_eq!(
        differences(&declared, broken.world()),
        vec![(NAMES_ITS_RESIDUE, CRUMBLING.to_owned(), DIRT.to_owned())],
        "the block this aims at names dirt as what it breaks into, not air — so a cell left \
         holding air here would be reading something other than the broken block's own \
         definition. Every block the base game ships breaks into air, which is why the fixture \
         registers one that does not, and why this assertion can tell a definition being read \
         from a residue being assumed"
    );
    Ok(())
}

#[test]
fn a_break_against_a_block_whose_definition_declares_it_unbreakable_leaves_it_where_it_is()
-> TestResult {
    let chamber = one_of_each();
    let declared = chamber.build()?;
    let refusing = after_a_break(&chamber, standing(FACING_THE_INDESTRUCTIBLE))?;
    let accepting = after_a_break(&chamber, standing(FACING_THE_BREAKABLE))?;

    assert_eq!(
        (
            differences(&declared, refusing.world()),
            differences(&declared, accepting.world())
        ),
        (
            Vec::new(),
            vec![(BESIDE_IT, CRUMBLING.to_owned(), DIRT.to_owned())]
        ),
        "a definition declaring itself unbreakable cannot be broken, so every cell of the \
         fixture still holds what it was declared with. The second half is the control that \
         stops that being satisfied by a simulation which breaks nothing at all: the identical \
         request, at the identical distance, against the breakable block standing beside it in \
         the same fixture, has to leave the block that one names. Deleting the check that reads \
         `breakable` empties the first cell and fails the first half"
    );
    Ok(())
}

#[test]
fn a_break_against_a_block_whose_definition_names_no_residue_empties_its_cell() -> TestResult {
    let chamber = one_ordinary_block_over_water();
    let declared = chamber.build()?;
    let broken = after_a_break(&chamber, standing(FACING_IT))?;

    assert_eq!(
        differences(&declared, broken.world()),
        vec![(EMPTIED, STONE.to_owned(), WATER.to_owned())],
        "naming no residue means the cell is emptied, not that the block cannot be broken — and \
         reading it the second way would make every block the base game ships indestructible, \
         since none of them names one. What a cell holds when it holds nothing is the world's \
         own word, which is why this fixture is filled with water: an implementation naming air \
         outright, or falling back to the first block the registry registered, leaves air here \
         and is caught, where against an air-filled world all three would have agreed"
    );
    Ok(())
}

#[test]
fn the_fixture_registry_numbers_the_base_air_block_first_and_the_unbreakable_block_after_it()
-> TestResult {
    let registry = fixture_registry()?;
    let first_registered = registry.definition(BlockId::from_raw(0))?;
    let indestructible = registry.id_of(&BlockName::parse(UNBREAKABLE)?)?;

    assert_eq!(
        (
            first_registered.name.as_str(),
            indestructible == BlockId::from_raw(0)
        ),
        (AIR, false),
        "the `fixture:` overlay applies over base content, never instead of it, and that ordering \
         is what a residue falling back to the first registered block would land on. It is what \
         makes the water-filled scenario above able to discriminate that fallback at all: built \
         from the overlay alone this registry would number one of the fixture's own blocks 0, and \
         which block that is would decide whether the fallback showed up — a property of the \
         registry's construction deciding what a scenario can prove"
    );
    Ok(())
}

/// A floor, and one solid block whose definition names dirt as its residue.
fn one_block_that_names_its_residue() -> BlockChamber {
    floored().cell(NAMES_ITS_RESIDUE, CRUMBLING)
}

/// A floor, one solid block that cannot be broken, and one that can, standing
/// the same distance away on the next row of z.
fn one_of_each() -> BlockChamber {
    floored()
        .cell(CANNOT_BE_BROKEN, UNBREAKABLE)
        .cell(BESIDE_IT, CRUMBLING)
}

/// A world whose emptiness is water rather than air, with one ordinary shipped
/// block standing in it.
///
/// Water is not solid, so it is a background the player moves through exactly as
/// it would move through air, and the floor is what it stands on. What the choice
/// buys is stated in this file's own header: it is the only way this scenario can
/// tell "the world's empty block" from "air" and from "whatever the registry
/// numbered first".
fn one_ordinary_block_over_water() -> BlockChamber {
    filled_with(WATER).cell(EMPTIED, STONE)
}

/// Air everywhere, with one layer of floor for the player to stand on.
fn floored() -> BlockChamber {
    filled_with(AIR)
}

/// One layer of floor for the player to stand on, and `background` everywhere
/// else — which is also what this world means by a cell holding nothing.
fn filled_with(background: &'static str) -> BlockChamber {
    BlockChamber::filled_with(COLUMNS, background).run(
        at(0, FLOOR_LAYER, 0),
        at(16, FLOOR_LAYER + 1, 16),
        STONE,
    )
}

/// A player standing still on the floor at `feet`, looking level along +x.
fn standing(feet: Vec3) -> PlayerState {
    PlayerState {
        position: feet,
        velocity: Vec3::ZERO,
        yaw: ALONG_THE_ROW,
        pitch: 0.0,
        on_ground: true,
    }
}

/// One tick over a fresh build of `chamber`, asking for no movement and one
/// break.
fn after_a_break(
    chamber: &BlockChamber,
    player: PlayerState,
) -> Result<Simulation, Box<dyn Error>> {
    let mut simulation = Simulation::new(player, chamber.build()?);
    simulation.advance(TickIntent {
        movement: MovementIntent::default(),
        action: Some(ActionIntent::Break),
    });
    Ok(simulation)
}
