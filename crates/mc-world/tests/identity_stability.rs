//! What a section says it holds when it is asked to describe itself, and what
//! that description means when it is read back somewhere else.
//!
//! A runtime id is dense, registry-local and reassigned the moment the block set
//! changes; a palette position means nothing at all outside the one section that
//! minted it. Neither can be a block's identity, because a world that stored
//! either would start reporting different blocks the day a mod is added or
//! removed — the player-visible failure is that placed blocks turn into other
//! blocks after an update. So the exported form carries namespaced names, and the
//! tests below are what make that claim falsifiable rather than merely stated.
//!
//! The two round-trip scenarios could pass vacuously if the registries they use
//! happened to agree, so each is paired with an assertion that the runtime ids
//! genuinely differ between them. Those pairs are the point: without them, "the
//! same blocks come back" is a claim about two identical registries.
//!
//! Import refuses rather than guesses. A name nothing is registered under and a
//! palette position past the end of the palette are both cases where an
//! arbitrary substitution would look like success and corrupt a world quietly.
//!
//! **A cell holding nothing is carried in the description as a palette entry
//! naming nothing**, so it survives a round trip the same way a block does. The
//! one thing that entry is not is a name: nothing is registered for it, nothing
//! could be, and an import that demanded one would refuse every section with a
//! hole in it.

mod common;

use std::error::Error;

use common::{
    NOTHING, TestResult, at, contents_at_every_position, described, generated_block_name,
    registry_of,
};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::section::{
    Contents, ImportError, PaletteIndex, Section, SectionData, VOXELS_PER_SECTION,
};

const AIR: &str = "base:air";
const STONE: &str = "base:stone";
const GRASS: &str = "base:grass";
const DIRT: &str = "base:dirt";
const WATER: &str = "base:water";

/// The order the blocks of the round-tripped section were registered in when it
/// was built.
const ORIGINAL_ORDER: [&str; 3] = [AIR, STONE, GRASS];

/// The same three blocks, registered in a different order — which is the whole
/// of what a game update does to a registry.
const REORDERED_ORDER: [&str; 3] = [GRASS, AIR, STONE];

/// The palette position the hand-built voxel data names, and the length of the
/// palette that does not have it.
const POSITION_PAST_THE_END: u16 = 3;

/// One voxel fewer than a section has.
const A_VOXEL_SHORT: usize = VOXELS_PER_SECTION - 1;

/// What a section's palette holds, in the order it holds them — a block by name,
/// and [`NOTHING`] for the entry that names none.
fn palette_names(section: &Section) -> Vec<String> {
    section.palette().map(described).collect()
}

/// What a description's palette holds, in the order it names them.
fn described_palette(exported: &SectionData) -> Vec<String> {
    exported
        .palette
        .iter()
        .map(|contents| described(contents.as_ref()))
        .collect()
}

/// How many of the blocks in `held` are `block`.
fn count_of(held: &[String], block: &str) -> usize {
    held.iter().filter(|entry| entry.as_str() == block).count()
}

/// `count` voxels, every one of them naming palette position `position`.
fn voxels_naming(position: u16, count: usize) -> Vec<PaletteIndex> {
    (0..count).map(|_| PaletteIndex::new(position)).collect()
}

/// A registry holding the three blocks of the round-tripped section and five
/// more, with the three registered third, fourth and fifth rather than first.
///
/// Five of the eight are the blocks this repository ships and three are invented,
/// which is what a world looks like after a mod is installed: the blocks that
/// were already there keep their names and lose their numbers.
fn grown_registry() -> Result<BlockRegistry, Box<dyn Error>> {
    let invented: Vec<String> = (0..3).map(generated_block_name).collect();
    let mut names = vec![DIRT, WATER, GRASS, AIR, STONE];
    names.extend(invented.iter().map(String::as_str));
    registry_of(&names)
}

/// A section holding air at one corner, stone at the opposite one and grass
/// everywhere else: what it holds at each of its 4096 positions, and the
/// description it exports.
///
/// Three distinct blocks and two of them at a single named position each, so a
/// round trip that lost a palette position, transposed two of them or dropped the
/// voxel data cannot come back looking the same.
fn a_mixed_section() -> Result<(Vec<String>, SectionData), Box<dyn Error>> {
    let registry = registry_of(&ORIGINAL_ORDER)?;
    let mut section = Section::filled(&BlockName::parse(GRASS)?, &registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(AIR)?, &registry)?;
    section.set_block(at(15, 15, 15), &BlockName::parse(STONE)?, &registry)?;
    Ok((contents_at_every_position(&section)?, section.export()?))
}

/// The refusal an import produced, or an explanation of why asserting on it
/// would have been vacuous.
fn refusal(outcome: Result<Section, ImportError>) -> Result<ImportError, Box<dyn Error>> {
    match outcome {
        Ok(accepted) => Err(format!(
            "this import must be refused, or the assertion below asserts nothing; it produced a \
             section holding {:?}",
            palette_names(&accepted)
        )
        .into()),
        Err(refused) => Ok(refused),
    }
}

#[test]
fn exporting_a_section_names_its_blocks_in_the_order_its_palette_holds_them() -> TestResult {
    let registry = registry_of(&ORIGINAL_ORDER)?;
    let mut section = Section::filled(&BlockName::parse(STONE)?, &registry)?;
    section.set_block(at(0, 0, 0), &BlockName::parse(AIR)?, &registry)?;

    let exported = section.export()?;

    let names = described_palette(&exported);
    assert_eq!(
        names,
        vec![STONE.to_owned(), AIR.to_owned()],
        "the exported palette is this section's own, in the order this section built it: \
         stone was the fill and air arrived afterwards. The registry registered air first \
         and the alphabet puts it first too, so an export that reported either order would \
         be describing something other than the section it was asked about"
    );
    Ok(())
}

#[test]
fn a_section_reimported_where_the_blocks_were_registered_in_another_order_holds_the_same_blocks()
-> TestResult {
    let (original, exported) = a_mixed_section()?;

    let reimported = Section::import(&exported, &registry_of(&REORDERED_ORDER)?)?;

    let held = contents_at_every_position(&reimported)?;
    assert_eq!(
        (
            held == original,
            held.len(),
            count_of(&held, AIR),
            count_of(&held, STONE),
            count_of(&held, GRASS),
        ),
        (true, 4096, 1, 1, 4094),
        "the registry the section comes back under registered the same three blocks in a \
         different order, which is what a game update does. Every one of the 4096 voxels \
         has to hold the block it held before, because a player's placed blocks are not \
         allowed to become other blocks when the game's block set is rearranged"
    );
    Ok(())
}

#[test]
fn the_reordered_registry_gives_stone_a_different_runtime_id_from_the_original() -> TestResult {
    let stone = BlockName::parse(STONE)?;

    let ids = (
        registry_of(&ORIGINAL_ORDER)?.id_of(&stone)?.get(),
        registry_of(&REORDERED_ORDER)?.id_of(&stone)?.get(),
    );

    assert_eq!(
        ids,
        (1, 2),
        "the round trip above proves nothing unless the two registries genuinely disagree \
         about what stone's number is — if they assigned the same ids, a section that had \
         stored ids all along would have come back looking correct"
    );
    Ok(())
}

#[test]
fn a_section_reimported_where_further_blocks_were_registered_first_holds_the_same_blocks()
-> TestResult {
    let (original, exported) = a_mixed_section()?;

    let reimported = Section::import(&exported, &grown_registry()?)?;

    let held = contents_at_every_position(&reimported)?;
    assert_eq!(
        (
            held == original,
            held.len(),
            count_of(&held, AIR),
            count_of(&held, STONE),
            count_of(&held, GRASS),
        ),
        (true, 4096, 1, 1, 4094),
        "this registry does not merely reorder the three blocks the section holds, it \
         registers five others around them — a mod being installed. The section still holds \
         exactly what it held, because what it stored was names"
    );
    Ok(())
}

#[test]
fn the_grown_registry_gives_air_a_different_runtime_id_from_the_original() -> TestResult {
    let air = BlockName::parse(AIR)?;
    let grown = grown_registry()?;

    let ids = (
        registry_of(&ORIGINAL_ORDER)?.id_of(&air)?.get(),
        grown.id_of(&air)?.get(),
    );

    assert_eq!(
        ids,
        (0, 3),
        "air held the very first runtime id in the registry the section was written under \
         and holds the fourth in the one it came back under. A section that had stored the \
         number 0 would now be reporting whatever block the new registry numbered 0"
    );
    Ok(())
}

#[test]
fn a_section_holding_both_empty_cells_and_blocks_comes_back_holding_the_same_contents() -> TestResult
{
    let registry = registry_of(&[STONE, GRASS])?;
    let mut section = Section::empty();
    section.set_block(at(0, 0, 0), &BlockName::parse(STONE)?, &registry)?;
    section.set_block(at(15, 15, 15), &BlockName::parse(GRASS)?, &registry)?;
    let original = contents_at_every_position(&section)?;

    let reimported = Section::import(&section.export()?, &registry)?;

    let held = contents_at_every_position(&reimported)?;
    assert_eq!(
        (
            held == original,
            held.len(),
            count_of(&held, NOTHING),
            count_of(&held, STONE),
            count_of(&held, GRASS),
        ),
        (true, 4096, 4094, 1, 1),
        "emptiness is a palette entry like any other, so a description carries it and an \
         import reads it back. A description that dropped the entry naming no block would \
         leave 4094 voxels pointing at a position the palette no longer has, and the two \
         named blocks would come back at the wrong ones"
    );
    Ok(())
}

#[test]
fn a_description_of_a_section_holding_empty_cells_imports_with_no_block_registered_for_them()
-> TestResult {
    let registry = registry_of(&[STONE])?;
    let mut section = Section::empty();
    section.set_block(at(0, 0, 0), &BlockName::parse(STONE)?, &registry)?;

    let reimported = Section::import(&section.export()?, &registry)?;

    assert_eq!(
        (palette_names(&reimported), registry.registered_count()),
        (vec![NOTHING.to_owned(), STONE.to_owned()], 1),
        "the registry holds exactly one block, and the description holds two entries — the \
         second of which names nothing. Import checks that every entry naming a block is \
         registered and asks nothing at all about the one that names none, because there is \
         no name to ask about: no registry could ever hold one"
    );
    Ok(())
}

#[test]
fn importing_a_section_naming_a_block_the_registry_does_not_hold_is_refused_naming_it() -> TestResult
{
    let exported =
        Section::filled(&BlockName::parse(STONE)?, &registry_of(&ORIGINAL_ORDER)?)?.export()?;
    let without_stone = registry_of(&[AIR, GRASS])?;

    let refused = refusal(Section::import(&exported, &without_stone))?;

    let ImportError::UnknownBlock { name } = &refused else {
        return Err(format!("expected an unknown-block refusal, got {refused:?}").into());
    };
    assert_eq!(
        name.as_str(),
        STONE,
        "the block this section is made of is not registered any more, and there is no \
         honest answer to what its voxels hold. Substituting the nearest thing would hand \
         a player a world quietly made of something else"
    );
    Ok(())
}

#[test]
fn importing_voxel_data_naming_a_palette_position_that_is_not_there_is_refused_naming_it()
-> TestResult {
    let palette = vec![
        Contents::Holds(BlockName::parse(STONE)?),
        Contents::Holds(BlockName::parse(GRASS)?),
        Contents::Holds(BlockName::parse(AIR)?),
    ];
    let mut indices = voxels_naming(0, VOXELS_PER_SECTION);
    *indices
        .last_mut()
        .ok_or("a section's worth of voxels has a last one")? =
        PaletteIndex::new(POSITION_PAST_THE_END);
    let exported = SectionData { palette, indices };

    let refused = refusal(Section::import(&exported, &registry_of(&ORIGINAL_ORDER)?))?;

    let ImportError::PaletteIndexOutOfRange { index, palette_len } = &refused else {
        return Err(format!("expected a palette-position refusal, got {refused:?}").into());
    };
    assert_eq!(
        (*index, *palette_len),
        (POSITION_PAST_THE_END, 3),
        "a palette naming three blocks has positions 0, 1 and 2, so position 3 names \
         nothing. Reading it as some other position, or as the last entry, would put a \
         block into the world that nothing ever asked for"
    );
    Ok(())
}

/// Guard. `SectionData`'s fields are public, so a description carrying anything
/// other than a whole section's worth of voxels is something a caller can
/// genuinely hand over — a truncated read, a partial write, a hand-built value —
/// and an import that filled in the difference would produce a section that looks
/// complete and is not.
#[test]
fn importing_voxel_data_that_is_not_a_whole_sections_worth_is_refused_naming_how_many_it_found()
-> TestResult {
    let exported = SectionData {
        palette: vec![Contents::Holds(BlockName::parse(STONE)?)],
        indices: voxels_naming(0, A_VOXEL_SHORT),
    };

    let refused = refusal(Section::import(&exported, &registry_of(&ORIGINAL_ORDER)?))?;

    let ImportError::WrongVoxelCount { found, expected } = &refused else {
        return Err(format!("expected a voxel-count refusal, got {refused:?}").into());
    };
    assert_eq!(
        (*found, *expected),
        (A_VOXEL_SHORT, VOXELS_PER_SECTION),
        "a section is 4096 voxels and nothing else, so a description one voxel short is \
         not a section: padding it or accepting it short would silently invent the missing \
         voxel"
    );
    Ok(())
}
