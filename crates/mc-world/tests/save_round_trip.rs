//! A save reloads the world that was saved — every cell of it, including the
//! cells that hold nothing.
//!
//! **The comparison is against the world the save was written from, cell by
//! cell, and never against "it was not refused".** A loader that hands back an
//! empty world satisfies `Ok` exactly as well as a correct one does, and an
//! empty world is precisely what a load that quietly gave up produces. So every
//! test here reads what came back at every position the world has and says where
//! the two first disagree.
//!
//! **Emptiness is part of what has to survive, and it has three possible
//! answers rather than two.** A cell that held nothing must come back holding
//! nothing — not a block, and not "there is no such cell". The third answer is
//! the one a shrunken world gives, and it is the one a comparison that only
//! walked the loaded world's own positions would never see. Nothing names
//! nothing, so a stored cell distinguishes empty from "holds the table's *n*th
//! name" without the table reserving an entry for it.
//!
//! **Fixture constraint no assertion can enforce.** Every world here holds
//! blocks at most of its positions rather than a handful. A world that was
//! almost entirely empty would agree with a loader that produced an empty world
//! at almost every position, and the assertion would be carried by the two or
//! three cells that differ — true, and far weaker than it reads.

mod common;

use std::collections::BTreeSet;

use common::assembled::{Voxel, assembled_world};
use common::persistence::{
    AGREES, STANDING_SOMEWHERE, answer_at, loaded_from, produced_from, save_in, world_at,
};
use common::{NOTHING, TestResult, generated_block, registry_of, registry_of_size};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::persistence::{self, Acceptance};
use mc_world::section::Contents;
use mc_world::world::{Extent, VoxelWorld, WorldPos};
use tempfile::TempDir;

/// The block every fixture world is mostly made of, and the three it is edited
/// with.
const BASE: &str = "fixture:stone";
const EDITS: [&str; 3] = ["fixture:andesite", "fixture:basalt", "fixture:chert"];

/// The three cells the edited world writes, chosen in three different sections
/// of the one column so that an edit lost to a section boundary is visible.
const EDITED: [WorldPos; 3] = [
    world_at(0, 0, 0),
    world_at(7, 100, 9),
    world_at(15, 255, 15),
];

/// The cell the vacated-entry world writes over and over.
const A_REVISITED_CELL: WorldPos = world_at(3, 3, 3);

/// The cell the emptied world clears, and its six neighbours.
const CLEARED: WorldPos = world_at(8, 40, 8);
const NEIGHBOURS: [WorldPos; 6] = [
    world_at(7, 40, 8),
    world_at(9, 40, 8),
    world_at(8, 39, 8),
    world_at(8, 41, 8),
    world_at(8, 40, 7),
    world_at(8, 40, 9),
];

/// How many columns a side the sixteen-column world spans, and the extent it
/// therefore has.
///
/// Derived rather than read back from the world: four columns of sixteen blocks
/// is sixty-four across, and a column is sixteen sections of sixteen blocks, so
/// two hundred and fifty-six tall. A test comparing the loaded extent against
/// the saved world's own would agree with a loader that returned the world
/// untouched and with one that returned nothing at all, since neither side would
/// be a number anybody wrote down.
const SIXTEEN_COLUMNS: u32 = 4;
const ITS_EXTENT: Extent = Extent {
    x: 64,
    y: 256,
    z: 64,
};

/// How many sections of the half-empty world hold a block, counting bottom-up,
/// and the height at which the empty half begins.
const FILLED_SECTIONS: usize = 8;
const THE_EMPTY_HALF_BEGINS_AT: u32 = 128;

/// How many distinct blocks the full-section world holds.
///
/// A compacted section's palette bound exactly: the other boundary the save's
/// two identifiers have between them, and the one a stored per-voxel position
/// has to reach.
const A_SECTIONS_WORTH: u32 = 4096;

/// How many blocks the registry of the four-block world declares, and which four
/// of them the world holds.
const DECLARED: u32 = 9;
const HELD: [u32; 4] = [5, 6, 7, 8];

/// A world of one column filled with `BASE`, saved and read back against
/// `registry` — the shape most fixtures here start from.
fn filled_with_base(registry: &BlockRegistry) -> Result<VoxelWorld, Box<dyn std::error::Error>> {
    Ok(VoxelWorld::filled(1, &BlockName::parse(BASE)?, registry)?)
}

/// What every position from height `from` up to height `up_to` answers, each
/// distinct answer once.
fn distinct_answers(world: &VoxelWorld, from: u32, up_to: u32) -> BTreeSet<String> {
    world
        .extent()
        .positions()
        .filter(|at| at.y >= from && at.y < up_to)
        .map(|at| answer_at(world, at))
        .collect()
}

#[test]
fn a_world_with_three_edited_cells_reads_back_holding_what_it_held_everywhere() -> TestResult {
    let directory = TempDir::new()?;
    let mut declared: Vec<&str> = EDITS.to_vec();
    declared.push(BASE);
    let registry = registry_of(&declared)?;
    let mut world = filled_with_base(&registry)?;
    for (at, name) in EDITED.into_iter().zip(EDITS) {
        world.set_block(at, &BlockName::parse(name)?, &registry)?;
    }
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    assert_eq!(
        produced_from(&path, &registry, Acceptance::OnlyUnchangedBlocks, &world)?,
        AGREES,
        "this is the whole promise, stated at the only granularity that means \
         anything: every cell of the world, not a sample of it. The three edits sit in three \
         different sections of the column, so an edit dropped at a section boundary — the easiest \
         thing to get wrong when sixteen sections are stacked back — shows up here rather than in \
         whichever fixture happened to write near a boundary"
    );
    Ok(())
}

#[test]
fn a_world_holding_palette_entries_no_voxel_refers_to_reads_back_unchanged() -> TestResult {
    let directory = TempDir::new()?;
    let mut declared: Vec<&str> = EDITS.to_vec();
    declared.push(BASE);
    let registry = registry_of(&declared)?;
    let mut world = filled_with_base(&registry)?;
    for name in EDITS {
        world.set_block(A_REVISITED_CELL, &BlockName::parse(name)?, &registry)?;
    }
    world.set_block(A_REVISITED_CELL, &BlockName::parse(BASE)?, &registry)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    assert_eq!(
        produced_from(&path, &registry, Acceptance::OnlyUnchangedBlocks, &world)?,
        AGREES,
        "one cell was written four times over, so the section's palette carries three entries no \
         voxel refers to any more. The save stores the minimal form, which renumbers every \
         surviving entry — and a renumbering that lost its way would put a plausible block in \
         every cell of the section rather than an obviously wrong one, which is exactly the \
         failure a comparison at every position catches and a spot check does not"
    );
    Ok(())
}

#[test]
fn a_world_of_sixteen_columns_reports_the_extent_it_had_before_it_was_saved() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[BASE])?;
    let world = VoxelWorld::filled(SIXTEEN_COLUMNS, &BlockName::parse(BASE)?, &registry)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    let loaded = loaded_from(&path, &registry, Acceptance::OnlyUnchangedBlocks)?;

    assert_eq!(
        loaded.extent(),
        ITS_EXTENT,
        "a save stores the footprint's side and the columns behind it, and the two have to agree \
         or the world comes back the wrong shape — every column present but addressed as though \
         the world were narrower, which reads as a world whose far half has moved rather than as \
         a load that failed. The extent is written out here rather than compared against the \
         world that was saved, so that a loader handing back its input would not satisfy it"
    );
    Ok(())
}

#[test]
fn a_section_whose_every_voxel_holds_a_different_block_reads_back_holding_each_of_them()
-> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of_size(A_SECTIONS_WORTH)?;
    let world = assembled_world(1, &registry, &|voxel: Voxel| {
        if voxel.section == 0 {
            generated_block(u32::try_from(voxel.offset).unwrap_or_default())
                .map_or(Contents::Empty, Contents::Holds)
        } else {
            Contents::Empty
        }
    })?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    assert_eq!(
        produced_from(&path, &registry, Acceptance::OnlyUnchangedBlocks, &world)?,
        AGREES,
        "four thousand and ninety-six distinct blocks in one section is a compacted palette at its \
         bound, and a stored per-voxel position at the top of the sixteen bits it is given. Every \
         voxel names a different entry, so a stored position that wrapped, truncated or drifted by \
         one puts a different — and perfectly plausible — block in the cell rather than failing"
    );
    Ok(())
}

#[test]
fn the_sections_of_a_world_that_held_nothing_still_hold_nothing_after_a_round_trip() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[BASE])?;
    let held = BlockName::parse(BASE)?;
    let world = assembled_world(1, &registry, &|voxel: Voxel| {
        if voxel.section < FILLED_SECTIONS {
            Contents::Holds(held.clone())
        } else {
            Contents::Empty
        }
    })?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    let loaded = loaded_from(&path, &registry, Acceptance::OnlyUnchangedBlocks)?;

    assert_eq!(
        (
            distinct_answers(&loaded, THE_EMPTY_HALF_BEGINS_AT, loaded.extent().y),
            distinct_answers(&loaded, 0, THE_EMPTY_HALF_BEGINS_AT)
        ),
        (
            BTreeSet::from([NOTHING.to_owned()]),
            BTreeSet::from([BASE.to_owned()])
        ),
        "the top eight sections have to answer `holds nothing` — not a block, and not `no such \
         cell`, which is what a world that came back short would answer and what a caller would \
         read as a corrupt save. The bottom eight are the control in the same breath: a loader \
         that produced an empty world would satisfy the first half of this on its own, and it is \
         the second half that says the emptiness above was stored rather than merely never filled"
    );
    Ok(())
}

#[test]
fn a_cell_that_was_emptied_reads_back_empty_between_six_untouched_neighbours() -> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[BASE])?;
    let mut world = filled_with_base(&registry)?;
    world.empty_at(CLEARED)?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    let loaded = loaded_from(&path, &registry, Acceptance::OnlyUnchangedBlocks)?;

    assert_eq!(
        (
            answer_at(&loaded, CLEARED),
            NEIGHBOURS.map(|at| answer_at(&loaded, at)).to_vec()
        ),
        (NOTHING.to_owned(), vec![BASE.to_owned(); 6]),
        "breaking a block is the edit this whole feature exists to preserve, and a cell that was \
         emptied is stored as emptiness rather than as an absence: the six neighbours are what \
         says the hole is exactly one cell wide. A stored palette position off by one would empty \
         a neighbour instead, and a loader that filled every empty cell with the block beside it \
         would put the broken block back"
    );
    Ok(())
}

#[test]
fn a_save_whose_four_blocks_are_all_registered_reads_back_the_world_it_was_saved_from() -> TestResult
{
    let directory = TempDir::new()?;
    let registry = registry_of_size(DECLARED)?;
    let held: Vec<BlockName> = HELD
        .into_iter()
        .map(generated_block)
        .collect::<Result<_, _>>()?;
    let world = assembled_world(1, &registry, &|voxel: Voxel| {
        held.get(voxel.offset & 3)
            .cloned()
            .map_or(Contents::Empty, Contents::Holds)
    })?;
    let path = save_in(&directory);
    persistence::save_world(&path, &world, STANDING_SOMEWHERE, &registry)?;

    assert_eq!(
        produced_from(&path, &registry, Acceptance::OnlyUnchangedBlocks, &world)?,
        AGREES,
        "the registry holds nine blocks and the world is made of four of them, sitting at runtime \
         ids five through eight — so a save that had stored an id rather than a name resolves to \
         the wrong four here, and a load that reported nothing missing while producing an empty \
         world would satisfy `no missing block` and fail this"
    );
    Ok(())
}
