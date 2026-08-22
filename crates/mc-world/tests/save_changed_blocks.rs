//! The three rows of the decision table that load, asserted against the world
//! they produce.
//!
//! A save records what each block it names was declared to be, and a load
//! compares. Three of the four outcomes are loadable: a block whose behaviour
//! changed, where the player has said to load anyway; a block whose texture
//! alone changed, which nobody is asked about; and a block that has not changed
//! at all. This file is those three, and every one of them is asserted against
//! **the world the load produced, compared against the world it was saved
//! from** — never against "it was not refused".
//!
//! **That is not a stylistic preference, it is the whole reason these three sit
//! here rather than beside the refusals.** A loader that hands back an empty
//! world was not refused either. And the permissive half of the acceptance
//! decision has no other falsifier at all: with the refusal logic broken so that
//! it ignored the player's decision entirely, every test in the phase before this
//! one stayed green, because every one of them names a save that is refused
//! anyway by a block nobody holds. The first test below is the only thing in the
//! suite that can tell the two decisions apart.
//!
//! **Fixture constraints no assertion can enforce.**
//!
//! - The changed fixtures vary **solidity** and nothing else. Solidity is
//!   behaviour — a block that was solid and is not any more is a world whose
//!   ground you fall through — while a texture is appearance, which is a
//!   different row of the table and is never a refusal. A fixture varying the
//!   texture would be describing a retexture and would assert the opposite of
//!   what it claims.
//! - The retextured fixture varies **the texture** and nothing else, for the
//!   same reason read the other way.
//! - The unchanged fixture builds its two registries as two separate instances
//!   from the same declarations, so "unchanged" is a comparison of two things
//!   rather than a comparison of one thing with itself.

mod common;

use std::error::Error;

use common::persistence::{AGREES, STANDING_SOMEWHERE, produced_from, save_in};
use common::{FIXTURE_ORIGIN, TestResult, registry_declaring, registry_from, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, Acceptance};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The two blocks whose declared behaviour changes between writing and reading,
/// and the four whose declarations do not change at all.
const REDECLARED: [&str; 2] = ["fixture:andesite", "fixture:basalt"];
const UNCHANGED: [&str; 4] = [
    "fixture:andesite",
    "fixture:basalt",
    "fixture:chert",
    "fixture:dacite",
];

/// The block whose texture alone changes, and the two textures it is declared
/// with.
const RETEXTURED: &str = "fixture:andesite";
const ONE_TEXTURE: &str = "fixture:pale";
const ANOTHER_TEXTURE: &str = "fixture:dark";

/// Where the blocks of a fixture world sit, beyond the block it is filled with.
const CELLS: [WorldPos; 4] = [
    world_cell(1, 1, 1),
    world_cell(2, 40, 3),
    world_cell(4, 120, 5),
    world_cell(6, 240, 7),
];

/// A world position, spelled out.
const fn world_cell(x: u32, y: u32, z: u32) -> WorldPos {
    WorldPos { x, y, z }
}

/// A registry declaring each of `names` with the solidity given.
fn declaring(names: &[&str], solid: bool) -> Result<BlockRegistry, Box<dyn Error>> {
    let declared: Vec<(&str, bool)> = names.iter().map(|name| (*name, solid)).collect();
    registry_declaring(&declared)
}

/// A world of one column filled with the first of `held`, carrying each of the
/// others at a cell of its own.
///
/// Filled rather than sprinkled: a world that was nearly all empty would agree
/// with a loader producing an empty world nearly everywhere, and the comparison
/// would be carried by a handful of cells.
fn a_world_of(held: &[&str], registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn Error>> {
    let fill = held
        .first()
        .ok_or("a fixture world needs a block to be made of")?;
    let mut world = VoxelWorld::filled(1, &BlockName::parse(fill)?, registry)?;
    for (at, name) in CELLS.into_iter().zip(held.iter().copied()) {
        world.set_block(at, &BlockName::parse(name)?, registry)?;
    }
    Ok(world)
}

#[test]
fn a_save_whose_two_blocks_were_redeclared_produces_its_world_when_the_player_accepts() -> TestResult
{
    let directory = TempDir::new()?;
    let written_against = declaring(&REDECLARED, true)?;
    let world = a_world_of(&REDECLARED, &written_against)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &written_against)?;
    let redeclared = declaring(&REDECLARED, false)?;

    assert_eq!(
        produced_from(&path, &redeclared, Acceptance::ChangedBlocksToo, &world)?,
        AGREES,
        "the player was told two blocks are not what they were, said load it anyway, and this is \
         what they must get: their world. Nothing else in the suite can tell the two decisions \
         apart — every other save that names a changed block is refused by a *missing* block as \
         well, which acceptance never covers — so a load that took the decision and ignored it \
         would be green everywhere but here. And `it was not refused` is not the assertion: an \
         empty world is not refused either, and it is exactly what a load that gave up produces"
    );
    Ok(())
}

#[test]
fn a_save_whose_block_was_only_retextured_produces_its_world_without_asking() -> TestResult {
    let directory = TempDir::new()?;
    let written_against = registry_from(FIXTURE_ORIGIN, &[(RETEXTURED, ONE_TEXTURE, true)])?;
    let world = a_world_of(&[RETEXTURED], &written_against)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &written_against)?;
    let retextured = registry_from(FIXTURE_ORIGIN, &[(RETEXTURED, ANOTHER_TEXTURE, true)])?;

    assert_eq!(
        produced_from(&path, &retextured, Acceptance::OnlyUnchangedBlocks, &world)?,
        AGREES,
        "a retextured block is the same block to stand on, and no decision was given here — the \
         load is the strictest one a caller can ask for and it still produces the world. This is \
         why the save records appearance and behaviour separately: one value for both would make \
         this file indistinguishable from a rebalanced one, and reporting every texture edit is \
         what teaches a player that a report means nothing, which destroys the only thing the \
         report is for"
    );
    Ok(())
}

#[test]
fn a_save_whose_four_blocks_are_unchanged_produces_its_world_without_acceptance() -> TestResult {
    let directory = TempDir::new()?;
    let written_against = registry_of(&UNCHANGED)?;
    let world = a_world_of(&UNCHANGED, &written_against)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &written_against)?;
    let read_against = registry_of(&UNCHANGED)?;

    assert_eq!(
        produced_from(
            &path,
            &read_against,
            Acceptance::OnlyUnchangedBlocks,
            &world
        )?,
        AGREES,
        "the positive control the whole comparison needs: four blocks, nothing changed about any \
         of them, no decision given, and the world comes back. A comparison that reported \
         `changed` for everything — a hash folded over a field it should not have, a registry read \
         at the wrong moment — satisfies every refusal in this group and fails only here. The two \
         registries are separate instances of the same declarations, so this is two things being \
         compared and not one thing agreeing with itself"
    );
    Ok(())
}
