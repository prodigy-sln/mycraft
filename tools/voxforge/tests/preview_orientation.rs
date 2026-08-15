//! Which way round the image comes out.
//!
//! This is the second of the two places the orientation hazard lives — the first
//! is the layer mapping, graded in `layer_geometry.rs` against its own fixtures,
//! deliberately not shared with these, because one fixture serving both would
//! let the two agree on the same error.
//!
//! **A raster has more symmetries than a grid**, so each fixture below is built
//! off the symmetry axis of every transformation its own scenario can reject,
//! and the ones it cannot reject are named:
//!
//! | Fixture | Vertical mirror | Horizontal mirror | Transpose | 180° rotation |
//! |---|---|---|---|---|
//! | [`TALL_COLUMN`] (rows) | caught | — | caught | caught |
//! | [`WIDE_ROW`] (columns) | — | caught | caught | caught |
//! | [`STACK`] (rows, `iso-fl`) | caught | — | caught | caught |
//!
//! The two `—` entries are covered by the other axis's fixture: a mirror lives
//! in the shared `right = normalize(d × w)` derivation, so a sign flip there
//! moves **every** view at once and [`WIDE_ROW`] reddens. A flip applied to one
//! view alone would escape all three, and nothing here can see it: D4's single
//! derivation is what excludes it, and the committed reference sheet and its
//! human sign-off are what confirm it.
//!
//! The two colours in each fixture are what a transformation has to move
//! relative to one another, so each fixture separates them along exactly **one**
//! image axis. Two voxels differing only in `y` share an image column under
//! every view but the two plan views, because `right` is perpendicular to the
//! `(0, 1, 0)` up hint by construction — which is why no isometric depth term
//! can legitimately reorder the stack.

mod common;

use common::preview::{EIGHT_PER_VOXEL, Paint, Placement, column_order, paints, row_order};
use common::{TestResult, assembled};
use voxforge::render::{View, render};
use voxforge::volume::StateSelection;

/// A `[3, 5, 1]` model whose one topmost voxel is red at `(2, 4, 0)` and whose
/// one bottom voxel is blue at `(2, 0, 0)` — the same column, so a transposed
/// raster puts them in the same row rather than in a plausible order.
///
/// The green column exists to make the image three voxels wide: a
/// single-column model has one column, and "the same column" then says nothing
/// at all. Its `y` span is 1 to 3, so the topmost and bottom voxels of the model
/// each stay unique.
const TALL_COLUMN: &str = r#"schema = 1
name = "base:tall_column"
scale = 16
size = [3, 5, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"g" = "base:jade"
"b" = "base:lapis"

[[layers]]
y = 0
grid = """
..b
"""

[[layers]]
y = 1
grid = """
g..
"""

[[layers]]
y = 2
grid = """
g..
"""

[[layers]]
y = 3
grid = """
g..
"""

[[layers]]
y = 4
grid = """
..r
"""
"#;

/// A `[5, 3, 1]` model whose one `+x` voxel is red at `(4, 0, 0)` and whose one
/// `−x` voxel is blue at `(0, 0, 0)` — the same row, so a transposed raster puts
/// them in the same column.
///
/// The green column raises the image to three voxels tall for the same reason
/// the other fixture is three wide.
const WIDE_ROW: &str = r#"schema = 1
name = "base:wide_row"
scale = 16
size = [5, 3, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"g" = "base:jade"
"b" = "base:lapis"

[[layers]]
y = 0
grid = """
b...r
"""

[[layers]]
y = 1
grid = """
..g..
"""

[[layers]]
y = 2
grid = """
..g..
"""
"#;

/// A `[1, 16, 1]` model holding red at `(0, 15, 0)` and blue at `(0, 0, 0)`, and
/// nothing else.
///
/// The fourteen planes between them declare no layer at all, which is a model
/// with two voxels rather than a model with fourteen empty ones.
const STACK: &str = r#"schema = 1
name = "base:stack"
scale = 16
size = [1, 16, 1]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"g" = "base:jade"
"b" = "base:lapis"

[[layers]]
y = 0
grid = """
b
"""

[[layers]]
y = 15
grid = """
r
"""
"#;

/// A `[4, 1, 4]` model holding blue at `(0, 0, 0)` and red at `(3, 0, 3)`, one
/// voxel tall.
///
/// The diagonal is what makes this fixture tell the four corner views apart.
/// `right` is `(1, 0, 1)/√2` for `iso-fl`, so the two voxels are 6/√2 apart
/// across the image; it is `(1, 0, −1)/√2` for `iso-fr` and `(−1, 0, 1)/√2` for
/// `iso-bl`, under either of which the two land in **exactly the same columns**;
/// and it is `(−1, 0, −1)/√2` for `iso-br`, which puts them the other way round.
/// One assertion therefore separates `iso-fl` from each of its three siblings
/// and from a sign flip in the shared derivation.
///
/// Neither voxel occludes the other: reaching one from the other along
/// `(1, −1, −1)` would need `x` and `z` to move in opposite directions, and here
/// they move together.
const DIAGONAL: &str = r#"schema = 1
name = "base:diagonal"
scale = 16
size = [4, 1, 4]
origin = [0, 0, 0]
slice = "y"

[palette]
"." = "empty"
"r" = "base:ruby"
"g" = "base:jade"
"b" = "base:lapis"

[[layers]]
y = 0
grid = """
b...
....
....
...r
"""
"#;

#[test]
fn the_material_at_the_top_of_the_model_reaches_lower_row_indices_than_the_one_at_its_foot()
-> TestResult {
    let volume = assembled(TALL_COLUMN, &StateSelection::default())?;

    assert_eq!(
        row_order(
            &render(&volume, &paints()?, View::Front, EIGHT_PER_VOXEL),
            Paint::Red,
            Paint::Blue
        ),
        Placement::FirstBeforeSecond,
        "row 0 is the top of the image and up in the model is up in the image, so the topmost voxel owns the lowest row indices"
    );
    Ok(())
}

#[test]
fn a_stack_seen_from_a_corner_keeps_its_upper_voxel_in_lower_row_indices() -> TestResult {
    let volume = assembled(STACK, &StateSelection::default())?;

    assert_eq!(
        row_order(
            &render(&volume, &paints()?, View::IsoFl, EIGHT_PER_VOXEL),
            Paint::Red,
            Paint::Blue
        ),
        Placement::FirstBeforeSecond,
        "the two voxels share one column of the image, so nothing an isometric projection does with depth can legitimately put the upper one lower"
    );
    Ok(())
}

#[test]
fn the_material_on_the_plus_x_side_reaches_higher_column_indices_than_the_one_opposite()
-> TestResult {
    let volume = assembled(WIDE_ROW, &StateSelection::default())?;

    assert_eq!(
        column_order(
            &render(&volume, &paints()?, View::Front, EIGHT_PER_VOXEL),
            Paint::Blue,
            Paint::Red
        ),
        Placement::FirstBeforeSecond,
        "`+x` in the model is to the right in a front view, so the `−x` voxel owns the lower column indices"
    );
    Ok(())
}

/// Additional coverage, beyond the scenario each other test here carries.
///
/// Nothing else in this phase reads a corner view's `right` vector at all: the
/// stack above grades only which way `up` points, and the two front-view tests
/// grade a vector `front` shares with no isometric view. A `right` that pointed
/// the wrong way along one corner — or the four corners wired to each other's
/// directions — would leave every one of the phase's twenty scenarios green,
/// while every isometric preview an agent then corrected itself against was
/// mirrored. This is the second witness that path has no other route to.
///
/// It stays clear of the deferred question. True isometric and 2:1 dimetric
/// differ only in the elevation of `d`, and `right = normalize(d × w)` with
/// `w = (0, 1, 0)` drops that component entirely, so the vector this asserts is
/// the same under either.
#[test]
fn a_corner_view_puts_the_voxel_furthest_along_both_horizontal_axes_at_the_right() -> TestResult {
    let volume = assembled(DIAGONAL, &StateSelection::default())?;

    assert_eq!(
        column_order(
            &render(&volume, &paints()?, View::IsoFl, EIGHT_PER_VOXEL),
            Paint::Blue,
            Paint::Red
        ),
        Placement::FirstBeforeSecond,
        "the front-left corner's rightward axis is `(1, 0, 1)/√2`, so the voxel far along both `+x` and `+z` is the one on the right of the image — under the other three corners these two share a column or swap places"
    );
    Ok(())
}
