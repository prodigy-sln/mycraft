//! What a section holds at each of its 4096 positions, and what it says when a
//! coordinate is not one of them.
//!
//! A section is the smallest thing in this engine that has to answer "what block
//! is here", and every consumer downstream of MVP 1 asks it: the mesher, physics,
//! editing and persistence. So the contract asserted here is deliberately blunt —
//! what was last written comes back, at every position, and a coordinate that is
//! not a position is refused by name rather than folded into one that is.
//!
//! The refusals matter as much as the reads. A section that wrapped x = 16 to
//! x = 0 would corrupt a neighbouring voxel silently, and a section that panicked
//! would take a 32-player tick loop down with it.
//!
//! **A cell holds a block or it holds nothing, and those are two of the answers
//! here; "there is no such cell" is a third and is never one of the first two.**
//! An out-of-bounds coordinate is refused by the axis it was on, and a cell that
//! holds nothing answers that it does — so a section that reported a coordinate
//! past its own edge as ordinary empty space would be dressing a storage fault as
//! a hole in the world.

mod common;

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use common::{
    NOTHING, TestResult, all_positions, at, contents_at_every_position, described, generated_block,
    registry_of, registry_of_size,
};
use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::section::{Axis, LocalPos, SECTION_SIZE, Section, SectionError};
use proptest::prelude::*;

const AIR: &str = "base:air";
const STONE: &str = "base:stone";
const GRASS: &str = "base:grass";

/// The position a coordinate one past the x bound folds onto if the fold is not
/// refused first: (16, 0, 0) and (0, 1, 0) share a linear index.
const COLLIDES_WITH_PAST_X: LocalPos = at(0, 1, 0);

/// The position a coordinate one past the y bound folds onto: (0, 16, 0) and
/// (0, 0, 1) share a linear index.
const COLLIDES_WITH_PAST_Y: LocalPos = at(0, 0, 1);

/// The one cell the filling and emptying scenarios write to. Nothing about it is
/// special beyond having all three coordinates different, so an accessor that
/// transposed two of them lands somewhere else.
const A_WRITTEN_CELL: LocalPos = at(3, 4, 5);

/// How many of the entries in `held` are `expected`.
fn count_of(held: &[String], expected: &str) -> usize {
    held.iter()
        .filter(|entry| entry.as_str() == expected)
        .count()
}

/// A section filled with stone, and the registry its blocks come from.
fn stone_filled_section() -> Result<(Section, BlockRegistry), Box<dyn Error>> {
    let registry = registry_of(&[AIR, STONE, GRASS])?;
    let section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    Ok((section, registry))
}

/// The refusal an access produced, or an explanation of why asserting on it
/// would have been vacuous.
fn refusal<T: Debug>(outcome: Result<T, SectionError>) -> Result<SectionError, Box<dyn Error>> {
    match outcome {
        Ok(accepted) => Err(format!(
            "this access must be refused, or the assertion below asserts nothing; it returned {accepted:?}"
        )
        .into()),
        Err(refused) => Ok(refused),
    }
}

/// The axis, value and limit an out-of-bounds refusal names.
fn out_of_bounds<T: Debug>(
    outcome: Result<T, SectionError>,
) -> Result<(Axis, u32, u32), Box<dyn Error>> {
    let refused = refusal(outcome)?;
    let SectionError::OutOfBounds { axis, value, limit } = refused else {
        return Err(format!("expected an out-of-bounds refusal, got {refused:?}").into());
    };
    Ok((axis, value, limit))
}

#[test]
fn a_filled_section_holds_its_fill_block_at_every_position() -> TestResult {
    let (section, _registry) = stone_filled_section()?;

    let held = contents_at_every_position(&section)?;

    let filled = count_of(&held, STONE);
    assert_eq!(
        (held.len(), filled),
        (4096, 4096),
        "a section created filled reports its fill block at every one of its voxels"
    );
    Ok(())
}

#[test]
fn two_written_blocks_replace_exactly_their_own_positions() -> TestResult {
    let (mut section, registry) = stone_filled_section()?;
    let grass = BlockName::parse(GRASS)?;

    section.set_block(at(0, 0, 0), &grass, &registry)?;
    section.set_block(at(15, 15, 15), &grass, &registry)?;

    let held = contents_at_every_position(&section)?;
    let written: Vec<LocalPos> = all_positions()
        .zip(&held)
        .filter(|(_, block)| block.as_str() == GRASS)
        .map(|(position, _)| position)
        .collect();
    let untouched = count_of(&held, STONE);
    assert_eq!(
        (written, untouched),
        (vec![at(0, 0, 0), at(15, 15, 15)], 4094),
        "a write lands at the position it names and nowhere else, so the two opposite \
         corners hold grass and the remaining 4094 voxels still hold the fill"
    );
    Ok(())
}

#[test]
fn a_write_past_the_x_bound_is_refused_naming_that_axis() -> TestResult {
    let (mut section, registry) = stone_filled_section()?;
    let grass = BlockName::parse(GRASS)?;

    let refused = out_of_bounds(section.set_block(at(16, 0, 0), &grass, &registry))?;

    assert_eq!(
        refused,
        (Axis::X, 16, 16),
        "x = 16 is one past the last column of a 16-wide section: wrapping it to x = 0 \
         would silently overwrite a different voxel, and panicking would end the tick"
    );
    Ok(())
}

#[test]
fn a_read_past_the_y_bound_is_refused_naming_that_axis() -> TestResult {
    let (mut section, registry) = stone_filled_section()?;
    // (0,0,1) is the voxel that an unchecked index computation hands back for
    // (0,16,0). It holds a different block here, so a section that answered from
    // there instead of refusing cannot pass by coincidence.
    section.set_block(at(0, 0, 1), &BlockName::parse(GRASS)?, &registry)?;

    let refused = out_of_bounds(section.block_at(at(0, 16, 0)))?;

    assert_eq!(
        refused,
        (Axis::Y, 16, 16),
        "y = 16 is not a position in a 16-tall section, and the voxel its index \
         collides with is not an answer to the question that was asked"
    );
    Ok(())
}

#[test]
fn a_read_past_the_z_bound_is_refused_naming_that_axis() -> TestResult {
    let (section, _registry) = stone_filled_section()?;

    let refused = out_of_bounds(section.block_at(at(0, 0, 16)))?;

    assert_eq!(
        refused,
        (Axis::Z, 16, 16),
        "z = 16 is one past the last layer of a 16-deep section"
    );
    Ok(())
}

#[test]
fn a_section_created_empty_holds_nothing_at_every_position() -> TestResult {
    let section = Section::empty();

    let held = contents_at_every_position(&section)?;

    assert_eq!(
        (held.len(), count_of(&held, NOTHING)),
        (4096, 4096),
        "a section that was never filled with anything holds nothing at all of its 4096 \
         cells — nothing being an answer the section gives rather than a block somebody had \
         to remember to put there"
    );
    Ok(())
}

#[test]
fn a_section_is_created_empty_against_a_registry_holding_no_block_while_a_filled_one_is_refused()
-> TestResult {
    let registry = BlockRegistry::new();
    let section = Section::empty();

    let held = contents_at_every_position(&section)?;
    let refused = refusal(Section::filled(&BlockName::parse(STONE)?, &registry))?;

    let SectionError::UnknownBlock { name } = &refused else {
        return Err(format!("expected an unknown-block refusal, got {refused:?}").into());
    };
    assert_eq!(
        (held.len(), count_of(&held, NOTHING), name.as_str()),
        (4096, 4096, STONE),
        "creating an empty section takes no registry and has no way to fail, because nothing \
         is not a block and there is nothing for a registry to know about it. Filling one with \
         a *name* still has to be checked, and this registry knows no name at all — so the two \
         halves are the same registry answering the two questions differently"
    );
    Ok(())
}

#[test]
fn a_read_past_the_x_bound_of_an_empty_section_is_refused_naming_that_axis() -> TestResult {
    let registry = registry_of(&[STONE, GRASS])?;
    let mut section = Section::empty();
    // The cell (16, 0, 0) folds onto holds a block, so neither "nothing is
    // there" nor "a block is there" is the answer to the question asked.
    section.set_block(COLLIDES_WITH_PAST_X, &BlockName::parse(STONE)?, &registry)?;

    let refused = out_of_bounds(section.block_at(at(16, 0, 0)))?;

    assert_eq!(
        refused,
        (Axis::X, 16, 16),
        "x = 16 is not a position in a 16-wide section, and a section that answered `nothing \
         is here` for it would be reporting a coordinate past its own edge as ordinary empty \
         space — the one confusion this whole storage shape exists to make impossible. The \
         voxel its index collides with holds stone, so neither honest-looking answer is \
         available by coincidence"
    );
    Ok(())
}

#[test]
fn emptying_a_cell_past_the_y_bound_is_refused_naming_that_axis_and_leaves_the_colliding_cell()
-> TestResult {
    let registry = registry_of(&[STONE, GRASS])?;
    let mut section = Section::empty();
    section.set_block(COLLIDES_WITH_PAST_Y, &BlockName::parse(GRASS)?, &registry)?;

    let refused = out_of_bounds(section.empty_at(at(0, 16, 0)))?;

    assert_eq!(
        (refused, described(section.block_at(COLLIDES_WITH_PAST_Y)?)),
        ((Axis::Y, 16, 16), GRASS.to_owned()),
        "(0, 16, 0) and (0, 0, 1) fold to the same linear index, so emptying without checking \
         every coordinate first empties the wrong cell instead of refusing. The second half is \
         what says so: the cell the index collides with still holds the block it was written \
         with, which a fold-then-empty leaves holding nothing"
    );
    Ok(())
}

#[test]
fn writing_one_block_into_an_empty_section_leaves_its_other_4095_cells_holding_nothing()
-> TestResult {
    let registry = registry_of(&[STONE, GRASS])?;
    let mut section = Section::empty();

    section.set_block(A_WRITTEN_CELL, &BlockName::parse(STONE)?, &registry)?;

    let held = contents_at_every_position(&section)?;
    let written: Vec<LocalPos> = all_positions()
        .zip(&held)
        .filter(|(_, entry)| entry.as_str() == STONE)
        .map(|(position, _)| position)
        .collect();
    assert_eq!(
        (written, count_of(&held, NOTHING), held.len()),
        (vec![A_WRITTEN_CELL], 4095, 4096),
        "all 4096 cells are read back and not merely the written one: a fixture that looked \
         only at the cell it wrote cannot tell a write that landed there from one that landed \
         everywhere, and both answer correctly at that cell"
    );
    Ok(())
}

#[test]
fn emptying_the_only_cell_of_a_section_holding_a_block_leaves_it_holding_nothing() -> TestResult {
    let registry = registry_of(&[STONE, GRASS])?;
    let mut section = Section::empty();
    section.set_block(A_WRITTEN_CELL, &BlockName::parse(STONE)?, &registry)?;

    section.empty_at(A_WRITTEN_CELL)?;

    let held = contents_at_every_position(&section)?;
    assert_eq!(
        (held.len(), count_of(&held, NOTHING)),
        (4096, 4096),
        "the one cell that held a block holds none afterwards, which puts the section back \
         where it started. An emptying that quietly did nothing leaves that cell holding stone \
         and 4095 rather than 4096 cells empty"
    );
    Ok(())
}

#[test]
fn a_write_carrying_a_runtime_id_the_registry_never_assigned_is_refused_naming_it() -> TestResult {
    const UNASSIGNED: u32 = 7;
    const REGISTERED: usize = 5;
    let registry = registry_of_size(5)?;
    let mut section = Section::filled(&generated_block(0)?, &registry)?;

    let refused =
        refusal(section.set_block_by_id(at(0, 0, 0), BlockId::from_raw(UNASSIGNED), &registry))?;

    let described = format!("{refused:?}");
    let SectionError::Registry(RegistryError::UnknownRuntimeId { id, registered }) = refused else {
        return Err(format!("expected an unknown-runtime-id refusal, got {described}").into());
    };
    assert_eq!(
        (id.get(), registered, section.palette().len()),
        (UNASSIGNED, REGISTERED, 1),
        "an id past the end of the registry is refused naming the id and how many \
         blocks there actually are, and the palette does not grow to accommodate it"
    );
    Ok(())
}

/// Guard. Nothing in the specification's own scenarios calls the `registry`
/// argument of the name-taking mutators, so an implementation that ignored it
/// would pass every one of them — on the most-copied signature in this feature.
/// This is what keeps that argument load-bearing: a section stores *registered*
/// blocks, and the moment to discover that a block is not registered is the edit,
/// not the mesh pass three ticks later.
#[test]
fn a_write_naming_a_block_the_registry_does_not_hold_is_refused_naming_it() -> TestResult {
    const UNREGISTERED: &str = "base:diamond";
    let (mut section, registry) = stone_filled_section()?;
    let unregistered = BlockName::parse(UNREGISTERED)?;

    let refused = refusal(section.set_block(at(1, 1, 1), &unregistered, &registry))?;

    let SectionError::UnknownBlock { name } = &refused else {
        return Err(format!("expected an unknown-block refusal, got {refused:?}").into());
    };
    assert_eq!(
        name.as_str(),
        UNREGISTERED,
        "a name no registry entry matches is refused naming it, rather than being \
         stored and becoming someone else's problem"
    );
    Ok(())
}

/// Guard, and it is not one of the specification's scenarios. The claim that a
/// section holding nothing and a section holding one block are the *same bytes*
/// of storage is what the argument that no rendered frame moves ultimately rests
/// on, and every scenario that reaches it does so three layers downstream —
/// through a mesher, a scene and a captured image — which is far too far away to
/// attribute a failure to. This asks it directly, of public API that already
/// exists.
///
/// Nothing here is a snapshot: a palette holding one entry needs no bits at all
/// to tell its voxels apart, zero bits over 4096 voxels is zero bytes, and the
/// only palette position either section can name is 0. Each figure is arithmetic
/// over the construction rather than a number read off a run.
#[test]
fn a_section_holding_nothing_and_one_holding_a_block_are_stored_the_same_way() -> TestResult {
    let (filled, _registry) = stone_filled_section()?;
    let empty = Section::empty();

    let widths = (empty.index_width_bits(), filled.index_width_bits());
    let bytes = (empty.index_storage_bytes(), filled.index_storage_bytes());
    let indices = empty.export()?.indices == filled.export()?.indices;

    assert_eq!(
        (widths, bytes, indices),
        ((0, 0), (0, 0), true),
        "one palette entry needs no index bits, no index bits need no bytes, and every voxel \
         of both sections names palette position 0 — so the two differ in what sits at that \
         position and in nothing else that is stored. If this ever parts, a moved golden frame \
         is the world's storage rather than the mesher's reading of it, and that attribution is \
         the whole reason this guard is here rather than left to the frame comparison"
    );
    Ok(())
}

/// How many distinct blocks the generated writes draw from.
///
/// Eighteen, not two: a section's index width steps 0 → 1 → 2 → 4 → 8 bits as its
/// palette grows past 1, 2, 4 and 16 entries, and a sequence that never reaches
/// 17 distinct blocks never re-packs the voxels it already wrote at the widest
/// step. Crossing every one of those transitions is what this property is for.
const POOL: u32 = 18;

/// An arbitrary in-bounds position.
fn a_position() -> impl Strategy<Value = LocalPos> {
    (0..SECTION_SIZE, 0..SECTION_SIZE, 0..SECTION_SIZE).prop_map(|(x, y, z)| at(x, y, z))
}

/// An arbitrary sequence of writes in which every one of the [`POOL`] blocks
/// appears at least once.
///
/// The covering half guarantees the palette reaches its full width; the free half
/// supplies the overwrites, repeats and collisions that are the interesting part.
/// Shuffling the two together is what stops the covering writes from all landing
/// first, which would make the order anything but arbitrary.
fn write_sequence() -> impl Strategy<Value = Vec<(LocalPos, u32)>> {
    let covering = proptest::collection::vec(a_position(), POOL as usize)
        .prop_map(|positions| positions.into_iter().zip(0..POOL).collect::<Vec<_>>());
    let further = proptest::collection::vec((a_position(), 0..POOL), 0..200);
    (covering, further)
        .prop_map(|(mut writes, extra)| {
            writes.extend(extra);
            writes
        })
        .prop_shuffle()
}

/// Applies `writes` to a section and reports the first position whose block
/// disagrees with a plain map of what was last written there.
fn first_disagreement(writes: &[(LocalPos, u32)]) -> Result<Option<String>, Box<dyn Error>> {
    let registry = registry_of_size(POOL)?;
    let fill = generated_block(0)?;
    let mut section = Section::filled(&fill, &registry)?;
    let mut last_written: HashMap<(u32, u32, u32), String> = HashMap::new();
    for (position, block) in writes {
        let name = generated_block(*block)?;
        section.set_block(*position, &name, &registry)?;
        last_written.insert(
            (position.x, position.y, position.z),
            name.as_str().to_owned(),
        );
    }
    for position in all_positions() {
        let held = described(section.block_at(position)?);
        let expected = last_written
            .get(&(position.x, position.y, position.z))
            .map_or(fill.as_str(), String::as_str);
        if held != expected {
            return Ok(Some(format!(
                "({}, {}, {}) holds `{held}` where `{expected}` was last written",
                position.x, position.y, position.z
            )));
        }
    }
    Ok(None)
}

proptest! {
    // Sixty-four cases rather than the default: each one builds an 18-block
    // registry, replays up to 218 writes and reads all 4096 voxels back, and an
    // integration test that runs for seconds stops being run.
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_section_reports_the_block_most_recently_written_at_every_position(
        writes in write_sequence()
    ) {
        let disagreement = first_disagreement(&writes)
            .map_err(|failure| TestCaseError::fail(failure.to_string()))?;

        prop_assert!(
            disagreement.is_none(),
            "every position holds what was last written there, or the fill where nothing \
             was: {disagreement:?}"
        );
    }
}
