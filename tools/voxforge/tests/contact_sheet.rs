//! Ten views in one image, and how a reader is told which tile is which.
//!
//! The sheet carries **no rendered text**. An earlier draft asserted "at least
//! one non-background pixel inside each label region", which is green for glyphs
//! that are garbage, transposed, or all identical — a count cannot see shape.
//! The tile order is declared and fixed, so the mapping is printed instead, and
//! a printed line either names its tile or does not.
//!
//! Every verdict here is enumerated rather than an absence, and each carries a
//! `WrongTileCount` arm for the same reason: a sheet that declares no tiles at
//! all would otherwise satisfy "every tile matches its own render" and "every
//! pixel outside a tile is background" by having nothing to check.

mod common;

use std::collections::BTreeSet;

use common::preview::{EIGHT_PER_VOXEL, Paint, halved_on_x, paints, pixels};
use common::{TestResult, assembled};
use voxforge::material::MaterialTable;
use voxforge::render::{ContactSheet, Pixel, Preview, TileRect, View, contact_sheet, render};
use voxforge::volume::{StateSelection, Volume};

/// A solid `[4, 6, 2]` model, blue on its `−x` half and red on its `+x` half.
///
/// Three different extents and two different materials, so no two of its ten
/// views are the same picture and a tile placed under the wrong view's name is
/// a difference rather than a coincidence.
fn subject() -> String {
    halved_on_x((4, 6, 2), Paint::Blue, Paint::Red)
}

/// Whether every tile of a sheet holds what that view renders on its own.
#[derive(Debug, PartialEq, Eq)]
enum Tiles {
    /// Every declared tile is exactly its own view's render.
    EveryTileMatchesItsView,
    /// The sheet declares a different number of tiles than there are views.
    WrongTileCount(usize),
    /// A tile's rectangle is not the shape that view renders to.
    Misshapen {
        /// Which view.
        view: &'static str,
        /// What that view renders to.
        expected: (u32, u32),
        /// What the tile reserves.
        found: (u32, u32),
    },
    /// A tile's pixels differ from that view's own render.
    Differs {
        /// Which view.
        view: &'static str,
        /// Where in the tile, as column and row within it.
        at: (u32, u32),
    },
}

/// Whether a sheet's printed mapping accounts for every tile.
#[derive(Debug, PartialEq, Eq)]
enum Legend {
    /// Every tile has a line naming it and its grid position, and no line is
    /// left over.
    EveryTileNamed,
    /// The sheet declares a different number of tiles than there are views.
    WrongTileCount(usize),
    /// No line names this tile.
    TileUnnamed {
        /// Which view.
        view: &'static str,
        /// Its grid column.
        column: u32,
        /// Its grid row.
        row: u32,
    },
    /// A line belongs to no tile.
    LineUnaccounted(String),
}

/// Whether everything of a sheet that is not a tile is background.
#[derive(Debug, PartialEq, Eq)]
enum Backdrop {
    /// Every pixel outside every tile is fully transparent.
    EveryPixelOutsideTilesClear,
    /// The sheet declares a different number of tiles than there are views.
    WrongTileCount(usize),
    /// Two tiles claim one pixel, so "outside every tile" means nothing.
    TilesOverlap {
        /// Its column.
        column: u32,
        /// Its row.
        row: u32,
    },
    /// A pixel outside every tile carries a colour.
    Painted {
        /// Its column.
        column: u32,
        /// Its row.
        row: u32,
        /// What it holds.
        pixel: Pixel,
    },
}

/// Every canonical view rendered on its own, in canonical order.
fn single_views(volume: &Volume, materials: &MaterialTable) -> Vec<(View, Preview)> {
    View::ALL
        .iter()
        .map(|view| (*view, render(volume, materials, *view, EIGHT_PER_VOXEL)))
        .collect()
}

/// Every pixel position one tile occupies within the sheet.
fn tile_pixels(rect: TileRect) -> impl Iterator<Item = (u32, u32)> {
    (0..rect.height).flat_map(move |row| {
        (0..rect.width).map(move |column| (rect.left + column, rect.top + row))
    })
}

/// Whether one tile holds `own`.
fn tile_matches(sheet: &Preview, view: View, rect: TileRect, own: &Preview) -> Tiles {
    if (rect.width, rect.height) != (own.width(), own.height()) {
        return Tiles::Misshapen {
            view: view.as_str(),
            expected: (own.width(), own.height()),
            found: (rect.width, rect.height),
        };
    }
    let differing = (0..rect.height)
        .flat_map(|row| (0..rect.width).map(move |column| (column, row)))
        .find(|(column, row)| {
            sheet.pixel(rect.left + column, rect.top + row) != own.pixel(*column, *row)
        });
    differing.map_or(Tiles::EveryTileMatchesItsView, |at| Tiles::Differs {
        view: view.as_str(),
        at,
    })
}

/// Whether every tile of `sheet` holds its own view's render.
fn tiles_hold_their_views(
    sheet: &ContactSheet,
    volume: &Volume,
    materials: &MaterialTable,
) -> Tiles {
    if sheet.tiles().len() != View::ALL.len() {
        return Tiles::WrongTileCount(sheet.tiles().len());
    }
    let alone = single_views(volume, materials);
    sheet
        .tiles()
        .iter()
        .map(
            |(view, rect)| match alone.iter().find(|(named, _)| named == view) {
                Some((_, own)) => tile_matches(sheet.image(), *view, *rect, own),
                None => Tiles::WrongTileCount(sheet.tiles().len()),
            },
        )
        .find(|verdict| *verdict != Tiles::EveryTileMatchesItsView)
        .unwrap_or(Tiles::EveryTileMatchesItsView)
}

/// Whether `line` names this tile: the view it holds, and where it sits.
fn names(line: &str, view: View, rect: TileRect) -> bool {
    line.contains(view.as_str())
        && line.contains(&format!("column {}", rect.column))
        && line.contains(&format!("row {}", rect.row))
}

/// Whether the sheet's printed mapping accounts for every tile and nothing else.
fn legend_accounts_for_every_tile(sheet: &ContactSheet) -> Legend {
    if sheet.tiles().len() != View::ALL.len() {
        return Legend::WrongTileCount(sheet.tiles().len());
    }
    let lines = sheet.legend();
    let unnamed = sheet
        .tiles()
        .iter()
        .find(|(view, rect)| !lines.iter().any(|line| names(line, *view, *rect)));
    if let Some((view, rect)) = unnamed {
        return Legend::TileUnnamed {
            view: view.as_str(),
            column: rect.column,
            row: rect.row,
        };
    }
    let spare = lines.iter().find(|line| {
        !sheet
            .tiles()
            .iter()
            .any(|(view, rect)| names(line, *view, *rect))
    });
    spare.map_or(Legend::EveryTileNamed, |line| {
        Legend::LineUnaccounted(line.clone())
    })
}

/// Every pixel of the sheet some tile claims, or the first one two tiles claim.
fn claimed(sheet: &ContactSheet) -> Result<BTreeSet<(u32, u32)>, (u32, u32)> {
    let mut marked = BTreeSet::new();
    for at in sheet
        .tiles()
        .iter()
        .flat_map(|(_, rect)| tile_pixels(*rect))
    {
        if !marked.insert(at) {
            return Err(at);
        }
    }
    Ok(marked)
}

/// Whether everything outside every tile is background.
fn outside_every_tile(sheet: &ContactSheet) -> Backdrop {
    if sheet.tiles().len() != View::ALL.len() {
        return Backdrop::WrongTileCount(sheet.tiles().len());
    }
    let marked = match claimed(sheet) {
        Ok(marked) => marked,
        Err((column, row)) => return Backdrop::TilesOverlap { column, row },
    };
    let painted = pixels(sheet.image())
        .find(|(column, row, pixel)| !marked.contains(&(*column, *row)) && !pixel.is_background());
    painted.map_or(
        Backdrop::EveryPixelOutsideTilesClear,
        |(column, row, pixel)| Backdrop::Painted { column, row, pixel },
    )
}

#[test]
fn every_tile_of_a_contact_sheet_holds_exactly_what_that_view_renders_on_its_own() -> TestResult {
    let volume = assembled(&subject(), &StateSelection::default())?;
    let materials = paints()?;

    assert_eq!(
        tiles_hold_their_views(
            &contact_sheet(&volume, &materials, EIGHT_PER_VOXEL),
            &volume,
            &materials
        ),
        Tiles::EveryTileMatchesItsView,
        "the sheet is a layout of the ten renders and not an eleventh rendering of the model, so nothing in it may differ from the view it claims to be"
    );
    Ok(())
}

#[test]
fn a_contact_sheet_reports_the_view_and_grid_position_of_each_of_its_tiles() -> TestResult {
    let volume = assembled(&subject(), &StateSelection::default())?;

    assert_eq!(
        legend_accounts_for_every_tile(&contact_sheet(&volume, &paints()?, EIGHT_PER_VOXEL)),
        Legend::EveryTileNamed,
        "nothing in the image says which tile is which, so the mapping the reader gets is the only one there is"
    );
    Ok(())
}

#[test]
fn every_pixel_of_a_contact_sheet_belonging_to_no_tile_is_left_at_the_background() -> TestResult {
    let volume = assembled(&subject(), &StateSelection::default())?;

    assert_eq!(
        outside_every_tile(&contact_sheet(&volume, &paints()?, EIGHT_PER_VOXEL)),
        Backdrop::EveryPixelOutsideTilesClear,
        "tiles of ten different shapes leave gaps between them, and a gap is background rather than whatever the layout happened to leave there"
    );
    Ok(())
}
