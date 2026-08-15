//! Ten views laid out in one image.
//!
//! **The sheet renders no text.** An earlier draft would have drawn a label over
//! each tile, and a pixel assertion about a label is green for glyphs that are
//! garbage, transposed, or all identical — a count cannot see shape. The tile
//! order is declared and fixed, so the mapping is *printed* instead, and a
//! printed line either names its tile or does not.
//!
//! The sheet is a layout of the ten renders and never an eleventh rendering of
//! the model: each tile is copied from that view's own image, so a tile and its
//! single-view render cannot drift apart.

use std::num::NonZeroU32;

use crate::material::MaterialTable;
use crate::render::{ContactSheet, Preview, TileRect, View, raster};
use crate::volume::Volume;

/// How many tiles the sheet places across before starting a new row.
const COLUMNS: u32 = 5;

/// Every canonical view of `volume`, tiled into one sheet.
#[must_use]
pub fn contact_sheet(
    volume: &Volume,
    materials: &MaterialTable,
    pixels_per_voxel: NonZeroU32,
) -> ContactSheet {
    let rendered: Vec<(View, Preview)> = View::ALL
        .iter()
        .map(|view| {
            (
                *view,
                raster::render(volume, materials, *view, pixels_per_voxel),
            )
        })
        .collect();
    let layout = Layout::of(&rendered);
    let tiles: Vec<(View, TileRect)> = rendered
        .iter()
        .enumerate()
        .map(|(index, (view, preview))| (*view, layout.rect_for(index, preview)))
        .collect();

    let mut image = Preview::blank(layout.width, layout.height);
    for ((_, preview), (_, rect)) in rendered.iter().zip(tiles.iter()) {
        copy_into(&mut image, preview, *rect);
    }
    ContactSheet { image, tiles }
}

/// Where each tile sits, given how big the ten renders came out.
///
/// A column is as wide as its widest tile and a row as tall as its tallest, so
/// ten differently shaped views tile without overlapping. What that leaves
/// between them is background rather than whatever the layout happened to hold.
struct Layout {
    /// The width of each tile column.
    columns: Vec<u32>,
    /// The height of each tile row.
    rows: Vec<u32>,
    /// How wide the whole sheet is.
    width: u32,
    /// How tall it is.
    height: u32,
}

impl Layout {
    /// The layout `rendered` needs.
    fn of(rendered: &[(View, Preview)]) -> Self {
        let mut columns = vec![0_u32; usize::try_from(COLUMNS).unwrap_or(1)];
        let mut rows = Vec::new();
        for (index, (_, preview)) in rendered.iter().enumerate() {
            note(&mut columns, &mut rows, index, preview);
        }
        let width = columns.iter().copied().sum();
        let height = rows.iter().copied().sum();
        Self {
            columns,
            rows,
            width,
            height,
        }
    }

    /// Where the tile at `index` sits, reserving exactly what `preview` fills.
    fn rect_for(&self, index: usize, preview: &Preview) -> TileRect {
        let (column, row) = cell_of(index);
        TileRect {
            column,
            row,
            left: self.offset(&self.columns, column),
            top: self.offset(&self.rows, row),
            width: preview.width(),
            height: preview.height(),
        }
    }

    /// How far in a given grid position begins.
    fn offset(&self, sizes: &[u32], position: u32) -> u32 {
        let position = usize::try_from(position).unwrap_or(0);
        sizes.iter().take(position).copied().sum()
    }
}

/// Records how much room the tile at `index` needs of its column and its row.
fn note(columns: &mut [u32], rows: &mut Vec<u32>, index: usize, preview: &Preview) {
    let (column, row) = cell_of(index);
    let column = usize::try_from(column).unwrap_or(0);
    let row = usize::try_from(row).unwrap_or(0);
    while rows.len() <= row {
        rows.push(0_u32);
    }
    if let Some(width) = columns.get_mut(column) {
        *width = (*width).max(preview.width());
    }
    if let Some(height) = rows.get_mut(row) {
        *height = (*height).max(preview.height());
    }
}

/// Which grid cell the tile at `index` occupies.
///
/// `div_euclid` and `rem_euclid` rather than `/` and `%`, because integer
/// division is denied workspace-wide and a tile layout is exactly the place a
/// silent truncation would go unnoticed.
fn cell_of(index: usize) -> (u32, u32) {
    let index = u32::try_from(index).unwrap_or(0);
    (index.rem_euclid(COLUMNS), index.div_euclid(COLUMNS))
}

/// Copies one view's render into its tile.
fn copy_into(sheet: &mut Preview, tile: &Preview, rect: TileRect) {
    for row in 0..tile.height() {
        copy_row(sheet, tile, rect, row);
    }
}

/// One row of a tile.
fn copy_row(sheet: &mut Preview, tile: &Preview, rect: TileRect, row: u32) {
    for column in 0..tile.width() {
        if let Some(pixel) = tile.pixel(column, row) {
            sheet.set(rect.left + column, rect.top + row, pixel);
        }
    }
}
