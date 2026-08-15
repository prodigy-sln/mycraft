//! Where a grid's rows and columns land in voxel space, and what a grid that
//! does not fit its part is told.
//!
//! A mirrored or axis-swapped reading is the worst defect this tool can have —
//! an agent self-corrects *against* the picture, so it would "fix" correct
//! geometry to match a broken view. Every fixture here is therefore built to sit
//! **off the symmetry axis of every transformation it must reject**, which is
//! two conditions and not one. Both are constraints no assertion can enforce.
//!
//! - **Non-cubic, with its filled cell off the diagonal**, so a row↔column
//!   transpose fails rather than mapping onto itself. A cubic model with its
//!   cell at row 0, column 0 agrees with a transposed reading.
//! - **An even number of rows** wherever a `z` or `x` layer prints `y`
//!   descending, so a vertical flip has no fixed point at all. `ey − 1 − r = r`
//!   is unsolvable in integers when `ey` is even. Every fixture here was three
//!   rows tall until 2026-08-15 with its cell on row 1, which is exactly the
//!   fixed point, and inverting the descending rule outright left all 49 of this
//!   phase's tests green.
//!
//! The first condition is the right defence against a transpose and no defence
//! at all against a flip, which is how the second hazard survived the audit that
//! closed the first. The preview raster is the other place both appear and it
//! gets its own fixtures on purpose — a fixture shared between the two would let
//! both agree on one error.

mod common;

use std::collections::BTreeSet;
use std::error::Error;

use common::{TestResult, all_named, as_crlf, loaded, refusal, unnamed};
use voxforge::format::{Extent, FilledCell, Part, Voxel};
use voxforge::name::MaterialKey;

/// The one material every fixture here paints with.
const MATERIAL: &str = "base:oak_plank";

/// A two-part model whose second part overrides the model's slice axis.
///
/// Both parts are `[5, 4, 2]` and both fill one off-diagonal cell, so the two
/// readings land on different voxels: `a` on `y` is a 2 × 5 grid whose row 1,
/// column 3 is `(3, 0, 1)`, and `b` on `z` is a 4 × 5 grid whose row 1,
/// column 3 is `(3, 2, 1)` — `y = 4 − 1 − 1`.
///
/// `b` carries the flip condition as well as the transpose one, being the only
/// part here that prints `y` descending. `a` does not, so its voxel is the same
/// as it was at `[4, 3, 2]`: a `y` layer's shape is `ez × ex` and neither
/// changed.
const TWO_PARTS_TWO_AXES: &str = r#"schema = 1
name = "base:probe"
scale = 16
slice = "y"

[palette]
"." = "empty"
"w" = "base:oak_plank"

[[parts]]
name = "a"
size = [5, 4, 2]
origin = [0, 0, 0]

[[parts]]
name = "b"
size = [5, 4, 2]
origin = [0, 0, 0]
slice = "z"
attach = { to = "a", at = [0, 0, 0] }

[[layers]]
part = "a"
y = 0
grid = """
.....
...w.
"""

[[layers]]
part = "b"
z = 1
grid = """
.....
...w.
.....
.....
"""
"#;

/// A one-part model of `size` sliced on `axis`, carrying `layers`.
fn model(size: &str, axis: &str, layers: &str) -> String {
    format!(
        r#"schema = 1
name = "base:probe"
scale = 16
size = {size}
origin = [0, 0, 0]
slice = "{axis}"

[palette]
"." = "empty"
"w" = "{MATERIAL}"
{layers}"#
    )
}

/// One layer of `axis` at `plane`, whose grid is `rows` joined by newlines.
fn layer(axis: &str, plane: u32, rows: &[&str]) -> String {
    let grid = rows.join("\n");
    format!(
        r#"
[[layers]]
{axis} = {plane}
grid = """
{grid}
"""
"#
    )
}

/// The filled cells of the model's only part.
fn cells_of(text: &str) -> Result<Vec<FilledCell>, Box<dyn Error>> {
    let document = loaded(text)?;
    let part = document
        .parts
        .first()
        .ok_or("the document declared no part at all")?;
    Ok(part.filled_cells())
}

/// The one cell a probe fixture fills.
fn only_cell(x: u32, y: u32, z: u32) -> Result<Vec<FilledCell>, Box<dyn Error>> {
    Ok(vec![FilledCell {
        position: Voxel { x, y, z },
        material: MaterialKey::parse(MATERIAL)?,
    }])
}

/// A grid of `rows` by `columns` holding one filled cell.
fn sparse_grid(rows: usize, columns: usize, filled_row: usize, filled_column: usize) -> String {
    let mut lines = Vec::new();
    for row in 0..rows {
        lines.push(sparse_row(
            columns,
            (row == filled_row).then_some(filled_column),
        ));
    }
    lines.join("\n")
}

/// One row of `columns` cells, filled at `filled` where there is one.
fn sparse_row(columns: usize, filled: Option<usize>) -> String {
    (0..columns)
        .map(|column| if filled == Some(column) { 'w' } else { '.' })
        .collect()
}

/// A 64-cube whose only layer is the topmost one, holding a single filled cell.
fn full_size_cube() -> String {
    let grid = sparse_grid(64, 64, 5, 7);
    model(
        "[64, 64, 64]",
        "y",
        &format!(
            r#"
[[layers]]
y = 63
grid = """
{grid}
"""
"#
        ),
    )
}

/// The planes a part's filled cells occupy on `y`.
fn planes_on_y(part: &Part) -> BTreeSet<u32> {
    part.filled_cells()
        .iter()
        .map(|cell| cell.position.y)
        .collect()
}

#[test]
fn a_y_slice_reads_its_first_row_as_z_zero_and_its_first_column_as_x_zero() -> TestResult {
    // A y layer is `ez` rows by `ex` columns: 2 by 5. Row 1 is z = 1 ascending,
    // column 3 is x = 3, and the plane is y = 0.
    let cells = cells_of(&model(
        "[5, 4, 2]",
        "y",
        &layer("y", 0, &[".....", "...w."]),
    ))?;

    assert_eq!(
        cells,
        only_cell(3, 0, 1)?,
        "row 1, column 3 of a y = 0 layer is the voxel at x = 3, y = 0, z = 1"
    );
    Ok(())
}

#[test]
fn a_z_slice_reads_its_first_row_as_the_top_of_the_model() -> TestResult {
    // A z layer is `ey` rows by `ex` columns: 4 by 5. Rows print y descending
    // from ey - 1, so row 1 is y = 4 - 1 - 1 = 2; column 3 is x = 3; the plane
    // is z = 1. Four rows is what stops the mirrored reading — which would
    // answer y = 1 — from agreeing with the correct one.
    let cells = cells_of(&model(
        "[5, 4, 2]",
        "z",
        &layer("z", 1, &[".....", "...w.", ".....", "....."]),
    ))?;

    assert_eq!(
        cells,
        only_cell(3, 2, 1)?,
        "a z layer prints y descending, so row 1 of a 4-tall model is y = 2"
    );
    Ok(())
}

#[test]
fn an_x_slice_reads_its_first_row_as_the_top_and_its_first_column_as_z_zero() -> TestResult {
    // An x layer is `ey` rows by `ez` columns: 4 by 2. Row 2 is
    // y = 4 - 1 - 2 = 1, column 1 is z = 1, and the plane is x = 2. Row 2 of 4
    // is off the diagonal as well as off the flip's axis, so neither
    // transformation maps this cell onto itself.
    let cells = cells_of(&model(
        "[5, 4, 2]",
        "x",
        &layer("x", 2, &["..", "..", ".w", ".."]),
    ))?;

    assert_eq!(
        cells,
        only_cell(2, 1, 1)?,
        "an x layer prints y descending across z, so row 2, column 1 of x = 2 is (2, 1, 1)"
    );
    Ok(())
}

#[test]
fn a_slice_axis_that_is_not_an_axis_is_refused_naming_the_three_that_are() -> TestResult {
    let fault = refusal(&model("[4, 3, 2]", "diagonal", ""))?;

    assert_eq!(
        (
            fault.field.as_deref(),
            unnamed(&fault, &["diagonal", "`x`", "`y`", "`z`"]),
        ),
        (Some("slice"), all_named()),
        "listing the accepted axes is what lets the author repair the file without the spec; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_part_declaring_its_own_slice_is_read_on_that_axis_while_its_siblings_are_not() -> TestResult {
    let document = loaded(TWO_PARTS_TWO_AXES)?;

    let inherited = document.part("a").map(Part::filled_cells);
    let overridden = document.part("b").map(Part::filled_cells);
    assert_eq!(
        (inherited, overridden),
        (Some(only_cell(3, 0, 1)?), Some(only_cell(3, 2, 1)?)),
        "the model's slice is a default a part may override, and the override reaches only that part"
    );
    Ok(())
}

#[test]
fn a_layer_short_of_a_row_is_refused_naming_the_rows_expected_and_found() -> TestResult {
    let fault = refusal(&model("[3, 3, 3]", "y", &layer("y", 0, &["w..", "..."])))?;

    assert_eq!(
        (
            fault.layer.map(|found| found.declaration),
            unnamed(&fault, &["expected 3 rows", "found 2"]),
        ),
        (Some(0), all_named()),
        "a grid that does not fit its extent is refused with both counts, not merely rejected; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_row_wider_than_its_extent_is_refused_naming_the_row_and_both_widths() -> TestResult {
    let fault = refusal(&model(
        "[3, 3, 3]",
        "y",
        &layer("y", 0, &["w..", "....", "..."]),
    ))?;

    assert_eq!(
        (
            fault.layer.map(|found| found.declaration),
            unnamed(&fault, &["row 1", "expected 3 columns", "found 4"]),
        ),
        (Some(0), all_named()),
        "one row being wrong is a different repair from the whole layer being wrong, so the row is named; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_layer_written_with_windows_line_endings_reads_as_the_same_voxels() -> TestResult {
    let text = model("[3, 3, 3]", "y", &layer("y", 0, &["...", "...", "w.."]));
    let expected = only_cell(0, 0, 2)?;

    let windows = cells_of(&as_crlf(&text))?;
    let unix = cells_of(&text)?;

    assert_eq!(
        (windows, unix),
        (expected.clone(), expected),
        "a carriage return is a line ending and never a cell, and both readings must be the right one rather than merely the same one"
    );
    Ok(())
}

#[test]
fn a_row_carrying_a_trailing_space_is_refused_as_one_column_too_wide() -> TestResult {
    let fault = refusal(&model(
        "[3, 3, 3]",
        "y",
        &layer("y", 0, &["w..", "... ", "..."]),
    ))?;

    assert_eq!(
        (
            fault.layer.map(|found| found.declaration),
            unnamed(&fault, &["row 1", "expected 3 columns", "found 4"]),
        ),
        (Some(0), all_named()),
        "trailing whitespace is invisible in an editor, which is exactly why it is refused rather than trimmed; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_grid_holding_a_character_outside_ascii_is_refused_naming_it_and_its_position() -> TestResult {
    let fault = refusal(&model(
        "[3, 3, 3]",
        "y",
        &layer("y", 0, &["...", "..é", "..."]),
    ))?;

    assert_eq!(
        (
            fault.layer.map(|found| found.declaration),
            unnamed(&fault, &["é", "row 1", "column 2", "ASCII"]),
        ),
        (Some(0), all_named()),
        "a palette key is one ASCII character, so a multi-byte character is refused before it is looked up; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_plane_no_layer_declares_holds_no_voxels() -> TestResult {
    let text = model(
        "[4, 3, 2]",
        "y",
        &format!(
            "{}{}",
            layer("y", 0, &["w...", "...."]),
            layer("y", 2, &["....", "...w"])
        ),
    );
    let document = loaded(&text)?;
    let part = document
        .parts
        .first()
        .ok_or("the document declared no part at all")?;

    assert_eq!(
        planes_on_y(part),
        BTreeSet::from([0, 2]),
        "an omitted layer index is an empty slab, and the planes that do hold voxels are exactly the two declared"
    );
    Ok(())
}

#[test]
fn two_layers_each_holding_one_cell_report_two_filled_voxels() -> TestResult {
    let cells = cells_of(&model(
        "[4, 3, 2]",
        "y",
        &format!(
            "{}{}",
            layer("y", 0, &["w...", "...."]),
            layer("y", 2, &["....", "...w"])
        ),
    ))?;

    // One filled character in each of two declared layers: 1 + 1, counted from
    // the fixture text rather than read off a run.
    assert_eq!(
        cells.len(),
        2,
        "the empty slab between the two declared layers contributes nothing to the count"
    );
    Ok(())
}

#[test]
fn two_layers_declaring_one_plane_are_refused_naming_the_repeated_index() -> TestResult {
    let fault = refusal(&model(
        "[4, 3, 2]",
        "y",
        &format!(
            "{}{}",
            layer("y", 0, &["w...", "...."]),
            layer("y", 0, &["....", "...w"])
        ),
    ))?;

    assert_eq!(
        (
            fault.layer.map(|found| (found.declaration, found.plane)),
            unnamed(&fault, &["y = 0", "two layers"]),
        ),
        (Some((1, Some(0))), all_named()),
        "one plane declared twice is ambiguous rather than additive, and the second declaration is where the repair goes; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_layer_indexed_past_its_extent_is_refused_naming_the_index_and_the_bound() -> TestResult {
    let fault = refusal(&model(
        "[3, 3, 3]",
        "y",
        &layer("y", 3, &["w..", "...", "..."]),
    ))?;

    assert_eq!(
        (
            fault.layer.map(|found| (found.declaration, found.plane)),
            unnamed(&fault, &["y = 3", "extent of 3"]),
        ),
        (Some((0, Some(3))), all_named()),
        "a plane one past the end is the commonest off-by-one in this format, so both the index and the bound are named; cause was: {}",
        fault.cause
    );
    Ok(())
}

#[test]
fn a_part_at_the_sixty_four_voxel_limit_loads_with_that_extent() -> TestResult {
    let document = loaded(&full_size_cube())?;
    let part = document
        .parts
        .first()
        .ok_or("the document declared no part at all")?;

    // Exactly one filled character was written into the grid above, so the
    // expected count is a property of the fixture rather than of a run.
    assert_eq!(
        (part.size, part.filled_cells().len()),
        (
            Extent {
                x: 64,
                y: 64,
                z: 64
            },
            1
        ),
        "64 is the limit and not one past it, and the top plane of a 64-cube is y = 63"
    );
    Ok(())
}
