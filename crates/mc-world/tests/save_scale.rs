//! What the save's own identifier has to reach, and what has to be decided
//! before a chunk is looked at.
//!
//! **Sixty-five thousand five hundred and thirty-seven, and not four thousand
//! and ninety-seven.** The identifier a save numbers its names by is bounded by
//! the distinct names across the whole save, which is a different bound entirely
//! from a section palette's — and mis-sizing it is the single highest-risk
//! mistake persistence can make. An identifier wrongly given a palette
//! position's width still addresses sixty-five thousand five hundred and
//! thirty-six of them, so a world of four thousand names would round-trip green
//! and prove nothing at all. One past that is the first count that discriminates,
//! and it is why this fixture is the size it is. **It is a declared runtime
//! budget exception, and trimming it would delete the only thing it says.**
//!
//! One column holds sixteen sections of four thousand and ninety-six cells,
//! which is sixty-five thousand five hundred and thirty-six — one short. So the
//! fixture spans two columns a side, four columns and a hundred and thirty-one
//! thousand cells, which is the smallest square footprint that can hold them. It
//! is built by describing sections and importing them, never by writing voxels
//! one at a time: the write path scans a palette per write and that route is
//! quadratic in the very number this fixture exists to be large.
//!
//! **The ordering claim is the other thing here.** A save naming blocks nobody
//! holds must be refused by naming them, whatever state the rest of the file is
//! in — that is what resolving the table up front is *for*, and a refusal that
//! reported the file's other problem instead would mean the resolution had moved
//! after the world was decoded.

mod common;

use common::assembled::{Voxel, assembled_world};
use common::handbuilt::{self, Entry, HandBuilt};
use common::persistence::{
    AGREES, STANDING_SOMEWHERE, produced_from, required_names, save_in, what_it_loaded,
};
use common::{TestResult, generated_block, registry_of, registry_of_size};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, Acceptance, LoadError};
use mc_world::section::Contents;
use mc_world::world::{VoxelWorld, WorldPos};
use std::error::Error;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// How many distinct blocks the large fixture holds, and how many columns a side
/// it takes to hold them.
const DISTINCT_BLOCKS: usize = 65_537;
const COLUMNS_A_SIDE: u32 = 2;

/// What the overwritten cell used to hold and what it holds now, and where it
/// sits.
const IT_USED_TO_HOLD: &str = "fixture:andesite";
const IT_HOLDS_NOW: &str = "fixture:basalt";
const THE_REVISITED_CELL: WorldPos = WorldPos { x: 5, y: 60, z: 9 };

/// The two blocks the cut-away save names, and what it records for each.
///
/// The recorded declarations are read back through the writer rather than
/// invented, because the control below needs the *other* registry to find them
/// unchanged: a pair of numbers chosen freely would make that registry report
/// them as changed, and the control would fail for a reason that has nothing to
/// do with the cut.
const NAMED_BY_THE_CUT_SAVE: [&str; 2] = ["fixture:quartz", "fixture:rhyolite"];

/// A world of one column made of what the revisited cell holds now, whose one
/// cell was written to the other block and then back again.
///
/// The other block is left in the section's palette with no voxel referring to
/// it, which is the shape the whole scenario is about.
fn a_world_whose_cell_was_overwritten(
    registry: &BlockRegistry,
) -> Result<VoxelWorld, Box<dyn Error>> {
    let holds_now = BlockName::parse(IT_HOLDS_NOW)?;
    let mut world = VoxelWorld::filled(1, &holds_now, registry)?;
    world.set_block(
        THE_REVISITED_CELL,
        &BlockName::parse(IT_USED_TO_HOLD)?,
        registry,
    )?;
    world.set_block(THE_REVISITED_CELL, &holds_now, registry)?;
    Ok(world)
}

/// A save in `directory` naming `table` and carrying no chunk data at all.
///
/// Not a real save cut short at an offset somebody chose, but a save whose world
/// record was never written — which is what "truncated to nothing" is, exactly,
/// and leaves nothing for an arithmetic mistake about where the table ends to
/// hide in.
fn a_save_cut_away(directory: &TempDir, table: &[Entry<'_>]) -> Result<PathBuf, Box<dyn Error>> {
    handbuilt::written(
        directory,
        "cut_away.mcw",
        HandBuilt {
            table,
            ..HandBuilt::default()
        },
    )
}

/// `names` as block names, in the order given.
fn block_names(names: &[&str]) -> Result<Vec<BlockName>, Box<dyn Error>> {
    let mut parsed = Vec::with_capacity(names.len());
    for name in names {
        parsed.push(BlockName::parse(name)?);
    }
    Ok(parsed)
}

/// What loading the save at `path` against `registry` answered, with any world
/// it produced reduced to its size.
fn loading(path: &Path, registry: &BlockRegistry) -> Result<String, LoadError> {
    what_it_loaded(persistence::load_world(
        path,
        registry,
        Acceptance::OnlyUnchangedBlocks,
    ))
}

/// The table a save of `names` carries, each name beside what a save the writer
/// produced records for it.
fn recording(
    names: &[&'static str],
    registry: &BlockRegistry,
) -> Result<Vec<Entry<'static>>, Box<dyn Error>> {
    let mut table = Vec::with_capacity(names.len());
    for name in names {
        let recorded = handbuilt::recorded_for(name, registry)?;
        table.push((*name, recorded.0, recorded.1));
    }
    Ok(table)
}

#[test]
fn a_world_of_65537_distinct_blocks_reads_back_holding_each_of_them() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of_size(u32::try_from(DISTINCT_BLOCKS)?)?;
    let world = assembled_world(COLUMNS_A_SIDE, &registry, &|voxel: Voxel| {
        let nth = voxel.nth();
        if nth < DISTINCT_BLOCKS {
            u32::try_from(nth)
                .ok()
                .and_then(|position| generated_block(position).ok())
                .map_or(Contents::Empty, Contents::Holds)
        } else {
            Contents::Empty
        }
    })?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    let required = persistence::requirements(&path)?;

    assert_eq!(
        (
            produced_from(&path, &registry, Acceptance::OnlyUnchangedBlocks, &world)?,
            required_names(&required).len()
        ),
        (AGREES.to_owned(), DISTINCT_BLOCKS),
        "one past what sixteen bits address, which is the first count that can tell a save-wide \
         identifier from a section palette's. Given a palette position's width, the last of these \
         names wraps to the first and every cell holding it comes back holding a different, \
         perfectly plausible block — no refusal, no corruption, just a world that is quietly not \
         the one that was saved. The name count is here as well because a count cannot see shape: \
         the fixture claims to hold this many distinct blocks and this is what says it does"
    );
    Ok(())
}

#[test]
fn a_cell_overwritten_before_saving_loads_against_a_registry_without_what_it_used_to_hold()
-> TestResult {
    let directory = TempDir::new()?;
    let written_against = registry_of(&[IT_USED_TO_HOLD, IT_HOLDS_NOW])?;
    let world = a_world_whose_cell_was_overwritten(&written_against)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &written_against)?;
    let since_uninstalled = registry_of(&[IT_HOLDS_NOW])?;

    let required = persistence::requirements(&path)?;

    assert_eq!(
        (
            produced_from(
                &path,
                &since_uninstalled,
                Acceptance::OnlyUnchangedBlocks,
                &world
            )?,
            required_names(&required)
        ),
        (AGREES.to_owned(), vec![IT_HOLDS_NOW.to_owned()]),
        "the world does not hold this block any more — a player put something else in that cell — \
         and the only trace of it is an entry in a palette that no voxel refers to. A save keeping \
         that entry needs a block the world is not made of, and the day the mod that defined it is \
         uninstalled the world refuses to load over something that is not in it. Writing the \
         minimal form is what makes this a load rather than a dead end, and it is the same claim \
         as `a vacated entry is not a name the save needs` made where it costs something"
    );
    Ok(())
}

#[test]
fn a_save_with_its_chunk_data_cut_away_is_refused_by_its_blocks_and_not_by_the_cut() -> TestResult {
    let directory = TempDir::new()?;
    let still_holding = registry_of(&NAMED_BY_THE_CUT_SAVE)?;
    let table = recording(&NAMED_BY_THE_CUT_SAVE, &still_holding)?;
    let path = a_save_cut_away(&directory, &table)?;
    let missing = block_names(&NAMED_BY_THE_CUT_SAVE)?;

    assert_eq!(
        (
            loading(&path, &BlockRegistry::new()),
            matches!(
                persistence::load_world(&path, &still_holding, Acceptance::OnlyUnchangedBlocks),
                Err(LoadError::Malformed { .. })
            )
        ),
        (
            Err(LoadError::Unresolvable {
                missing,
                changed: Vec::new()
            }),
            true
        ),
        "this file is wrong in two ways at once and the order it is refused in is the one property \
         the whole table exists for: the blocks it names are gone, and there is no chunk data \
         behind it at all. Refused by the two names, the player is told to put a mod back. Refused \
         by the missing world, they are told the file is broken and the mod they need is never \
         mentioned. The second half is what says the cut is real — the same file, read against a \
         registry that does hold both blocks, gets past the table and finds nothing behind it"
    );
    Ok(())
}
