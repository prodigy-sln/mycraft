//! Where a column-local coordinate lands, and what a column says when a
//! coordinate is not one it has.
//!
//! A column is sixteen sections stacked, and the only interesting thing it does is
//! decide which of them owns a given height and what that height is called once it
//! gets there. Both halves of that answer can be wrong independently, so the
//! heights read back below are chosen to separate them: one section too low, one
//! section too high, and the same local height in the wrong section all read
//! differently from the one that was written.
//!
//! The coordinate a column is created at is signed, because half of any world sits
//! at a negative x or z. A column that could not carry -2 would not fail loudly; it
//! would quietly be a different column.

mod common;

use std::error::Error;
use std::fmt::Debug;

use common::{TestResult, registry_of};
use mc_core::block::BlockRegistry;
use mc_core::id::BlockName;
use mc_world::column::{COLUMN_HEIGHT, ChunkColumn, ColumnCoordinate, ColumnPos};
use mc_world::section::{Axis, SectionError};

const AIR: &str = "base:air";
const STONE: &str = "base:stone";

/// A column-local position, spelled out.
const fn in_column(x: u32, y: u32, z: u32) -> ColumnPos {
    ColumnPos { x, y, z }
}

/// A column of air at (3, -2), and the registry its blocks come from.
fn air_filled_column() -> Result<(ChunkColumn, BlockRegistry), Box<dyn Error>> {
    let registry = registry_of(&[AIR, STONE])?;
    let column = ChunkColumn::filled(
        ColumnCoordinate { x: 3, z: -2 },
        &BlockName::parse(AIR)?,
        &registry,
    )?;
    Ok((column, registry))
}

/// The refusal an access produced, or an explanation of why asserting on it would
/// have been vacuous.
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
fn a_filled_column_holds_its_fill_block_at_its_lowest_and_highest_positions() -> TestResult {
    let (column, _registry) = air_filled_column()?;

    let held = (
        column.block_at(in_column(0, 0, 0))?.as_str().to_owned(),
        column.block_at(in_column(15, 255, 15))?.as_str().to_owned(),
    );

    assert_eq!(
        held,
        (AIR.to_owned(), AIR.to_owned()),
        "filling a column fills all sixteen of its sections, so the voxel in its bottom \
         corner and the voxel in its top corner both hold the fill — and the top one is \
         at y = 255 because a column is 256 blocks tall"
    );
    Ok(())
}

#[test]
fn a_column_reports_the_coordinate_it_was_created_at() -> TestResult {
    let (column, _registry) = air_filled_column()?;

    let coordinate = column.coordinate();

    assert_eq!(
        (coordinate.x, coordinate.z),
        (3, -2),
        "a column coordinate is signed on both axes: a column at z = -2 is an ordinary \
         column in any world, and a coordinate that could not carry it would place that \
         column somewhere else entirely"
    );
    Ok(())
}

#[test]
fn a_write_lands_in_the_section_that_owns_its_height_and_at_the_right_height_inside_it()
-> TestResult {
    let (mut column, registry) = air_filled_column()?;

    column.set_block(in_column(4, 17, 9), &BlockName::parse(STONE)?, &registry)?;

    let held = [
        in_column(4, 17, 9),
        in_column(4, 16, 9),
        in_column(4, 1, 9),
        in_column(4, 33, 9),
    ]
    .into_iter()
    .map(|position| {
        column
            .block_at(position)
            .map(|block| block.as_str().to_owned())
    })
    .collect::<Result<Vec<String>, SectionError>>()?;

    assert_eq!(
        held,
        vec![
            STONE.to_owned(),
            AIR.to_owned(),
            AIR.to_owned(),
            AIR.to_owned()
        ],
        "y = 17 is the second voxel of the second section. y = 16 is the first voxel of \
         that same section, y = 1 is the same height in the section below, and y = 33 is \
         the same height one section further up — a routing that dropped the section, \
         kept the whole height, or was off by one section would land on one of the three"
    );
    Ok(())
}

#[test]
fn a_read_above_the_top_of_a_column_is_refused_naming_that_axis() -> TestResult {
    let (column, _registry) = air_filled_column()?;

    let refused = out_of_bounds(column.block_at(in_column(0, COLUMN_HEIGHT, 0)))?;

    assert_eq!(
        refused,
        (Axis::Y, 256, 256),
        "a column is sixteen sections of sixteen voxels, so y = 256 is one past its top; \
         folding it back into the lowest section would report a voxel from the other end \
         of the world"
    );
    Ok(())
}

#[test]
fn a_write_above_the_top_of_a_column_is_refused_naming_that_axis() -> TestResult {
    let (mut column, registry) = air_filled_column()?;

    let refused = out_of_bounds(column.set_block(
        in_column(0, COLUMN_HEIGHT, 0),
        &BlockName::parse(STONE)?,
        &registry,
    ))?;

    assert_eq!(
        refused,
        (Axis::Y, 256, 256),
        "a write above the top of a column is refused rather than wrapped: a wrapped write \
         would silently destroy a block at the bottom of the world"
    );
    Ok(())
}

#[test]
fn a_read_past_the_x_bound_of_a_column_is_refused_naming_that_axis() -> TestResult {
    let (column, _registry) = air_filled_column()?;

    let refused = out_of_bounds(column.block_at(in_column(16, 0, 0)))?;

    assert_eq!(
        refused,
        (Axis::X, 16, 16),
        "a column is only as wide as one section, so x = 16 belongs to the neighbouring \
         column and is not this one's to answer for"
    );
    Ok(())
}
