//! What a save records about each block it names, beyond the name.
//!
//! A name resolving proves *a* block exists under it, not that it is the block
//! the world was built from. A mod updated, a mod forked, or a different mod
//! claiming the same name all load silently against a name-only check — so a
//! save records what each block it names was declared to be, and a load
//! compares.
//!
//! **Behaviour and appearance are recorded separately, and the split is the
//! point.** A block whose texture changed is the same block to stand on; a block
//! whose solidity, replaceability, breakability or drop changed is not.
//! Recording one value for both would make a retextured mod indistinguishable
//! from a rebalanced one, and the only safe answer to that ambiguity is to
//! prompt on every texture edit — which teaches a player to accept without
//! reading, and a prompt nobody reads destroys the only thing the recorded
//! declaration is for. The retexture test below is what makes the split
//! observable; nothing else asserts it is real.
//!
//! The expected hash in the version-1 test is **derived by hand from the
//! documented encoding and never taken from a run of the code under test**. That
//! requirement is what chose the hash function: a constant nobody can derive
//! independently is a snapshot of whatever the writer did the day it was
//! written, and it can never fail for the right reason afterwards.

mod common;

use std::error::Error;

use common::persistence::{
    STANDING_SOMEWHERE, declaration_of, save_in, saved_requirements, world_at, world_holding,
};
use common::{FIXTURE_ORIGIN, TestResult, registry_from, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, SaveError, SaveRequirements};
use mc_world::world::{VoxelWorld, WorldPos};
use tempfile::TempDir;

/// Four blocks whose declarations differ from each other in both halves: no two
/// share a texture, and solidity alternates so that neither half can be told
/// apart by the name alone.
const DECLARED: [(&str, &str, bool); 4] = [
    ("fixture:andesite", "fixture:andesite_face", true),
    ("fixture:basalt", "fixture:basalt_face", false),
    ("fixture:chert", "fixture:chert_face", true),
    ("fixture:diorite", "fixture:diorite_face", false),
];

/// Where each of [`DECLARED`] sits in the world holding them all, and the two
/// of those cells the one- and two-block fixtures use.
const A_CELL: WorldPos = world_at(1, 1, 1);
const ANOTHER_CELL: WorldPos = world_at(2, 3, 4);
const CELLS: [WorldPos; 4] = [
    A_CELL,
    ANOTHER_CELL,
    world_at(5, 8, 13),
    world_at(15, 200, 15),
];

/// The block whose texture is changed between two otherwise identical
/// registries, and the two textures it is given.
const RETEXTURED: &str = "fixture:andesite";
const FIRST_TEXTURE: &str = "fixture:andesite_face";
const SECOND_TEXTURE: &str = "fixture:andesite_reworked";

/// The block whose behaviour hash is pinned to a value derived by hand.
const PINNED: &str = "fixture:stone";

/// What version 1 of this format records for a block named `fixture:stone`
/// declared solid, not replaceable, breakable, and breaking into nothing.
///
/// **Derived by hand from the documented encoding**, in a scratch computation
/// sharing no code with the writer. The canonical input is the declared
/// behaviour — an input-version byte, the name, the three flags, and the absence
/// of a residue — encoded little-endian with variable-length lengths, which for
/// values this small is one byte each:
///
/// ```text
///   01                          input version 1
///   0d                          the name is 13 bytes long
///   66 69 78 74 75 72 65        f i x t u r e
///   3a                          :
///   73 74 6f 6e 65              s t o n e
///   01                          solid
///   00                          not replaceable
///   01                          breakable
///   00                          breaks into nothing
/// ```
///
/// Folded with FNV-1a 64 — start at `0xcbf2_9ce4_8422_2325`, and for each byte
/// exclusive-or it in and multiply by `0x0000_0100_0000_01b3`, wrapping — those
/// nineteen bytes give the value below. The fold was checked against FNV's own
/// published vectors (`""`, `"a"`, `"foobar"`) before it was pointed at this
/// input.
const VERSION_1_BEHAVIOUR_OF_PINNED: u64 = 0x5e9d_3089_5b2e_0d5f;

/// A world holding a block the registry it is saved against does not declare,
/// and the block that is missing from it.
const KNOWN_TO_BOTH: &str = "fixture:andesite";
const KNOWN_TO_ONE: &str = "fixture:basalt";

/// Every name [`DECLARED`] declares.
fn declared_names() -> Vec<&'static str> {
    DECLARED.into_iter().map(|(name, _, _)| name).collect()
}

/// What a save recorded about one block: its name, its declared behaviour and
/// its declared appearance.
type Recorded = (String, u64, u64);

/// What a save recorded, in the order it reports it.
fn recorded(requirements: &SaveRequirements) -> Vec<Recorded> {
    requirements
        .blocks()
        .iter()
        .map(|block| {
            (
                block.name.as_str().to_owned(),
                block.behaviour.get(),
                block.appearance.get(),
            )
        })
        .collect()
}

/// What a save of a world holding `name` alone records about it.
fn recorded_alone(name: &str, registry: &BlockRegistry) -> Result<Vec<Recorded>, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let world = world_holding(&[(A_CELL, name)], registry)?;
    Ok(recorded(&saved_requirements(&directory, &world, registry)?))
}

/// Each of [`DECLARED`] at its cell.
fn every_block_placed() -> Vec<(WorldPos, &'static str)> {
    CELLS.into_iter().zip(declared_names()).collect()
}

/// What a save of `world` records for `name`, against a registry declaring
/// [`RETEXTURED`] with `texture` and everything else as [`DECLARED`] has it.
fn declaration_recorded_for(
    name: &str,
    world: &VoxelWorld,
    texture: &'static str,
) -> Result<Option<(u64, u64)>, Box<dyn Error>> {
    let directory = TempDir::new()?;
    let registry = registry_from(FIXTURE_ORIGIN, &declared_texturing(texture))?;
    let required = saved_requirements(&directory, world, &registry)?;
    Ok(declaration_of(&required, name))
}

/// The same four declarations with [`RETEXTURED`] carrying `texture`.
fn declared_texturing(texture: &'static str) -> Vec<(&'static str, &'static str, bool)> {
    DECLARED
        .into_iter()
        .map(|(name, own_texture, is_solid)| {
            let given = if name == RETEXTURED {
                texture
            } else {
                own_texture
            };
            (name, given, is_solid)
        })
        .collect()
}

#[test]
fn a_save_records_against_each_name_the_declaration_that_block_carried() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_from(FIXTURE_ORIGIN, &DECLARED)?;
    let world = world_holding(&every_block_placed(), &registry)?;

    let together = recorded(&saved_requirements(&directory, &world, &registry)?);

    let mut apart = Vec::new();
    for name in declared_names() {
        apart.extend(recorded_alone(name, &registry)?);
    }
    assert_eq!(
        together, apart,
        "each of the four names comes back carrying what that block was declared to be, and the \
         oracle is the same four blocks saved one at a time: a report that recorded the right \
         four declarations against the wrong four names would agree with itself, agree with \
         every count, and be exactly the failure that lets a load compare a block against \
         somebody else's declaration"
    );
    Ok(())
}

#[test]
fn changing_only_a_block_texture_moves_its_recorded_appearance_and_leaves_its_behaviour()
-> TestResult {
    let world = world_holding(
        &every_block_placed(),
        &registry_from(FIXTURE_ORIGIN, &DECLARED)?,
    )?;

    let was = declaration_recorded_for(RETEXTURED, &world, FIRST_TEXTURE)?;
    let now = declaration_recorded_for(RETEXTURED, &world, SECOND_TEXTURE)?;

    assert_eq!(
        (
            was.map(|(behaviour, _)| behaviour) == now.map(|(behaviour, _)| behaviour),
            was.map(|(_, appearance)| appearance) == now.map(|(_, appearance)| appearance),
            was.is_some()
        ),
        (true, false, true),
        "the two registries differ in one block's texture and in nothing else, so the recorded \
         appearance has to move and the recorded behaviour has to stand still. This is the only \
         assertion that makes the split real: with one value for both, a retexture and a \
         rebalance are the same event, every texture edit prompts, and a player learns to accept \
         without reading"
    );
    Ok(())
}

#[test]
fn a_solid_breakable_block_that_breaks_into_nothing_records_the_version_1_behaviour() -> TestResult
{
    let directory = TempDir::new()?;
    let registry = registry_of(&[PINNED])?;
    let world = world_holding(&[(A_CELL, PINNED)], &registry)?;

    let required = saved_requirements(&directory, &world, &registry)?;

    assert_eq!(
        required.blocks().first().map(|block| block.behaviour.get()),
        Some(VERSION_1_BEHAVIOUR_OF_PINNED),
        "this is what version 1 of the format means by that declaration, and the value is derived \
         from the documented encoding rather than read off a run — a number copied out of the \
         first green run records whatever the writer did that day and pins nothing. Recording \
         one value today and another tomorrow makes every save in existence report its blocks as \
         changed, which is the prompt this whole mechanism exists to raise only when something \
         really did change"
    );
    Ok(())
}

#[test]
fn saving_a_world_holding_a_block_the_registry_does_not_declare_is_refused_by_name() -> TestResult {
    let directory = TempDir::new()?;
    let built_against = registry_of(&[KNOWN_TO_BOTH, KNOWN_TO_ONE])?;
    let world = world_holding(
        &[(A_CELL, KNOWN_TO_BOTH), (ANOTHER_CELL, KNOWN_TO_ONE)],
        &built_against,
    )?;
    let saved_against = registry_of(&[KNOWN_TO_BOTH])?;

    let saved = persistence::save_world(
        &save_in(&directory),
        &world,
        STANDING_SOMEWHERE,
        &saved_against,
    );

    assert_eq!(
        saved,
        Err(SaveError::UnknownBlock {
            name: BlockName::parse(KNOWN_TO_ONE)?
        }),
        "a world can legitimately hold a name the registry it is being saved against does not \
         declare — it was built against a different one — and there is nothing honest to record \
         for it. Inventing a declaration would write a save that reports itself unchanged the \
         next time it is opened against the registry that actually has the block"
    );
    Ok(())
}
