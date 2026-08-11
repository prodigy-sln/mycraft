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

mod common;

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use common::{
    TestResult, all_positions, at, blocks_at_every_position, generated_block, registry_of,
    registry_of_size,
};
use mc_core::block::{BlockId, BlockRegistry, RegistryError};
use mc_core::id::BlockName;
use mc_world::section::{Axis, LocalPos, SECTION_SIZE, Section, SectionError};
use proptest::prelude::*;

const AIR: &str = "base:air";
const STONE: &str = "base:stone";
const GRASS: &str = "base:grass";

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

    let held = blocks_at_every_position(&section)?;

    let filled = held.iter().filter(|block| block.as_str() == STONE).count();
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

    let held = blocks_at_every_position(&section)?;
    let written: Vec<LocalPos> = all_positions()
        .zip(&held)
        .filter(|(_, block)| block.as_str() == GRASS)
        .map(|(position, _)| position)
        .collect();
    let untouched = held.iter().filter(|block| block.as_str() == STONE).count();
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
        let held = section.block_at(position)?.as_str();
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
