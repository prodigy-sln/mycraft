//! What a stored world is refused for, once the decoder has handed it over.
//!
//! **Every refusal here is ours.** The library turning bytes into typed values is
//! treated as working, and no test in this suite asserts how it classifies
//! anything: each fixture below decodes perfectly well and is then turned away
//! because of what the decoded values *mean* — a column list that does not fill
//! the footprint it declares, a section describing the wrong number of voxels, a
//! voxel naming a palette position its section lacks, a palette entry naming a
//! table position the table lacks, and bytes left over after the world.
//!
//! **The fixtures are constructed and never truncated**, and that distinction is
//! the whole of the first one: a file merely cut short after fifteen columns
//! fails inside the decoder, which is a different thing failing for a different
//! reason. A world record declaring four columns to a side and carrying fifteen
//! complete columns encodes and decodes without complaint, and *we* refuse it.
//!
//! **Two of these are two scenarios and not one, and that is what the two-level
//! identifier buys.** A voxel reaches the save's table by way of its section's
//! palette, so a voxel naming a position its palette lacks and a palette entry
//! naming a name the table lacks are different failures at different levels —
//! and only the first can name a world position, because only the loader knows
//! which column, which section and which voxel it was looking at. A design
//! storing one table position per voxel would collapse them into one failure and
//! leave one of the two scenarios unreachable.

mod common;

use std::error::Error;
use std::path::{Path, PathBuf};

use common::handbuilt::{
    self, ALL_AT_THE_FIRST_ENTRY, Cell, Column, EMPTY_COLUMN, Entry, HandBuilt, Stored,
    VOXELS_PER_SECTION, World,
};
use common::persistence::{SAVE_FILE, what_it_loaded};
use common::{TestResult, registry_of};
use mc_core::block::BlockRegistry;
use mc_world::persistence::{self, Acceptance, LoadError};
use mc_world::section::ImportError;
use mc_world::world::WorldPos;
use tempfile::TempDir;

/// The footprint the fifteen-column fixture declares, and how many columns a
/// world of that side has.
const DECLARED_SIDE: u32 = 4;
const COLUMNS_IT_SHOULD_HAVE: usize = 16;
const COLUMNS_IT_HAS: usize = 15;

/// How many columns a side every other fixture here spans.
const ONE_COLUMN: u32 = 1;

/// How many voxels a section holds, and the two counts that are not that.
const VOXELS_A_SECTION_HOLDS: usize = VOXELS_PER_SECTION;
const ONE_SHORT: usize = VOXELS_A_SECTION_HOLDS - 1;
const ONE_TOO_MANY: usize = VOXELS_A_SECTION_HOLDS + 1;

/// A palette holding nothing, and one holding the table's first name.
const HOLDING_NOTHING: [Cell; 1] = [Cell::Empty];
const HOLDING_THE_FIRST_NAME: [Cell; 1] = [Cell::Holds(0)];

/// The one block the fixtures with a table name.
const NAMED: &str = "fixture:andesite";

/// Which section of the one column carries the voxel that names nothing, and the
/// voxel's own position inside it — one across, two up, three along, which is
/// linear position 801 counting x fastest, then y, then z.
const IN_SECTION: usize = 5;
const AT_OFFSET: usize = 1 + 16 * 2 + 256 * 3;

/// Where that voxel therefore sits in the world: the position inside the section
/// plus the section's own height, in a column at the world's origin.
const ITS_WORLD_POSITION: WorldPos = WorldPos {
    x: 1,
    y: 2 + 16 * IN_SECTION as u32,
    z: 3,
};

/// The palette position that voxel names, and how many its palette has.
const A_POSITION_THE_PALETTE_LACKS: u16 = 5;
const THE_PALETTE_HOLDS: usize = 1;

/// The table position a palette entry names, and how many the table has.
const A_NAME_THE_TABLE_LACKS: u32 = 7;
const THE_TABLE_HOLDS: usize = 1;

/// How many bytes the trailing-bytes fixture carries past its own end.
const LEFT_OVER: usize = 64;

/// A save in `directory` of one column stacking `sections`, needing `table`.
fn a_save_of(
    directory: &TempDir,
    sections: &[Stored<'_>],
    table: &[Entry<'_>],
) -> Result<PathBuf, Box<dyn Error>> {
    let columns = [Column { sections }];
    handbuilt::written(
        directory,
        SAVE_FILE,
        HandBuilt {
            table,
            world: Some(World {
                footprint_side: ONE_COLUMN,
                columns: &columns,
            }),
            ..HandBuilt::default()
        },
    )
}

/// Every byte of a save of one column holding nothing at all.
fn a_whole_save() -> Vec<u8> {
    let columns = [EMPTY_COLUMN];
    handbuilt::bytes_of(HandBuilt {
        world: Some(World {
            footprint_side: ONE_COLUMN,
            columns: &columns,
        }),
        ..HandBuilt::default()
    })
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

/// A section describing `count` voxels and holding nothing.
fn describing(count: usize) -> Vec<u16> {
    vec![0_u16; count]
}

#[test]
fn a_save_declaring_sixteen_columns_and_carrying_fifteen_is_refused_naming_both_counts()
-> TestResult {
    let directory = TempDir::new()?;
    let columns = [EMPTY_COLUMN; COLUMNS_IT_HAS];
    let path = handbuilt::written(
        &directory,
        SAVE_FILE,
        HandBuilt {
            world: Some(World {
                footprint_side: DECLARED_SIDE,
                columns: &columns,
            }),
            ..HandBuilt::default()
        },
    )?;

    assert_eq!(
        loading(&path, &BlockRegistry::new()),
        Err(LoadError::WrongColumnCount {
            expected: COLUMNS_IT_SHOULD_HAVE,
            found: COLUMNS_IT_HAS
        }),
        "fifteen perfectly good columns and a footprint that wants sixteen. Every column here \
         decodes, so nothing below this level has anything to complain about — and a load that \
         assembled what it was given would produce a world whose far corner is somewhere else \
         entirely, silently. Both counts are named because the one found says how much of the \
         file there is and the one expected says what it was supposed to be"
    );
    Ok(())
}

#[test]
fn a_section_describing_one_voxel_too_few_is_refused_naming_both_counts() -> TestResult {
    let directory = TempDir::new()?;
    let indices = describing(ONE_SHORT);
    let short = Stored {
        palette: &HOLDING_NOTHING,
        indices: &indices,
    };
    let path = a_save_of(&directory, &handbuilt::column_of(short, 0), &[])?;

    assert_eq!(
        loading(&path, &BlockRegistry::new()),
        Err(LoadError::Section(ImportError::WrongVoxelCount {
            found: ONE_SHORT,
            expected: VOXELS_A_SECTION_HOLDS
        })),
        "a section is a fixed volume, and a list of positions one shorter than that is not a \
         shorter section — it is a section with a voxel nobody described. Padding it out would \
         build a section that looks whole and holds a cell the file never mentioned, which is the \
         quietest way a corrupt save could put something in a player's world"
    );
    Ok(())
}

#[test]
fn a_section_describing_one_voxel_too_many_is_refused_naming_both_counts() -> TestResult {
    let directory = TempDir::new()?;
    let indices = describing(ONE_TOO_MANY);
    let over = Stored {
        palette: &HOLDING_NOTHING,
        indices: &indices,
    };
    let path = a_save_of(&directory, &handbuilt::column_of(over, 0), &[])?;

    assert_eq!(
        loading(&path, &BlockRegistry::new()),
        Err(LoadError::Section(ImportError::WrongVoxelCount {
            found: ONE_TOO_MANY,
            expected: VOXELS_A_SECTION_HOLDS
        })),
        "the other side of the same bound, and the side a reader is likelier to wave through: one \
         position too many can be read by taking the first four thousand and ninety-six and \
         ignoring the rest, which loads a world nobody wrote and leaves the file looking fine"
    );
    Ok(())
}

#[test]
fn a_voxel_naming_a_palette_position_its_section_lacks_is_refused_naming_the_world_position()
-> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[NAMED])?;
    let recorded = handbuilt::recorded_for(NAMED, &registry)?;
    let mut indices = ALL_AT_THE_FIRST_ENTRY.to_vec();
    if let Some(voxel) = indices.get_mut(AT_OFFSET) {
        *voxel = A_POSITION_THE_PALETTE_LACKS;
    }
    let odd = Stored {
        palette: &HOLDING_THE_FIRST_NAME,
        indices: &indices,
    };
    let sections = handbuilt::column_of(odd, IN_SECTION);
    let path = a_save_of(&directory, &sections, &[(NAMED, recorded.0, recorded.1)])?;

    assert_eq!(
        loading(&path, &registry),
        Err(LoadError::UnknownCellEntry {
            at: ITS_WORLD_POSITION,
            index: A_POSITION_THE_PALETTE_LACKS,
            palette_len: THE_PALETTE_HOLDS
        }),
        "one cell out of four thousand names a palette entry that is not there, and the position \
         is the only part of that a player can do anything with — `palette position 5 of 1` is \
         true of this file in four thousand places and identifies none of them. Naming it means \
         the loader has to know which column, which section and which voxel it was reading, which \
         is exactly the knowledge the section importer does not have"
    );
    Ok(())
}

#[test]
fn a_palette_entry_naming_a_table_position_the_table_lacks_is_refused_naming_the_identifier()
-> TestResult {
    let directory = TempDir::new()?;
    let registry = registry_of(&[NAMED])?;
    let recorded = handbuilt::recorded_for(NAMED, &registry)?;
    let palette = [Cell::Holds(A_NAME_THE_TABLE_LACKS)];
    let odd = Stored {
        palette: &palette,
        indices: &ALL_AT_THE_FIRST_ENTRY,
    };
    let sections = handbuilt::column_of(odd, 0);
    let path = a_save_of(&directory, &sections, &[(NAMED, recorded.0, recorded.1)])?;

    assert_eq!(
        loading(&path, &registry),
        Err(LoadError::UnknownNameId {
            id: A_NAME_THE_TABLE_LACKS,
            table_len: THE_TABLE_HOLDS
        }),
        "this is the level above the one before it: the palette entry is fine as a stored value \
         and names a table position the table does not have. Indexing on it is exactly the thing \
         a hostile file is written to make a reader do, and how many entries the table has is \
         what turns the refusal into a fact about the file rather than a number with no scale"
    );
    Ok(())
}

#[test]
fn a_save_carrying_bytes_after_its_world_is_refused_naming_where_it_should_have_ended() -> TestResult
{
    let directory = TempDir::new()?;
    let whole = a_whole_save();
    let should_have_ended_at = u64::try_from(whole.len())?;
    let mut trailed = whole.clone();
    trailed.extend(std::iter::repeat_n(0_u8, LEFT_OVER));
    let with_more = handbuilt::file_holding(&directory, "and_then_some.mcw", &trailed)?;
    let without = handbuilt::file_holding(&directory, SAVE_FILE, &whole)?;

    assert_eq!(
        (
            loading(&with_more, &BlockRegistry::new()),
            loading(&without, &BlockRegistry::new()).is_ok()
        ),
        (
            Err(LoadError::TrailingBytes {
                should_have_ended_at
            }),
            true
        ),
        "a save ends where the world ends, exactly. Sixty-four bytes past that is a file somebody \
         appended to — a smuggled payload, a botched merge of two saves, a download that overshot \
         — and reading the world out and shrugging at the rest would accept all three. The offset \
         is what says where the save a player wanted stopped. The second half is the control: the \
         same bytes without the sixty-four load, so this refusal is about what was added rather \
         than about a fixture that was never readable"
    );
    Ok(())
}
