//! Reading the table a modding page states a declaration's fields in.
//!
//! # One reader, however many directions are asked of it
//!
//! This is the same argument `quoted_refusals.rs` makes about the refusal
//! recogniser, one artefact over: a second copy of a markdown table parser is
//! the thing these guards exist to prevent, one level up — two instruments
//! agreeing with each other about what a row is, while neither agrees with the
//! pages. So the reader lives here and each binary that wants an answer out of
//! it asks its own question.
//!
//! # What a row is, and what it is deliberately not
//!
//! A row belongs to the field table when its first cell is fenced in backticks,
//! which is how every page writes a field name and how none of them writes prose.
//! The table is found by its **header**: a first column called `Field` beside an
//! `Absent means` and a `Bound` column. A page that renamed either of those is
//! found by nothing and reaches the caller's own error rather than answering with
//! an empty table — an absent instrument and a clean one must not look alike.

// Each binary that includes this uses a subset of it.
#![allow(dead_code)]

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// The extension a page is written with.
pub const PAGE_EXTENSION: &str = "md";

/// The first cell of the header row of the table that names every field.
pub const FIELD_COLUMN: &str = "Field";

/// The header of the column stating what leaving a field out means.
pub const ABSENT_MEANS_COLUMN: &str = "Absent means";

/// The header of the column stating what values a field may hold.
pub const BOUND_COLUMN: &str = "Bound";

/// One row of the guide's field table: the field it names, what it says leaving
/// that field out means, and the bound it states on the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabulatedRow {
    pub field: String,
    pub absent_means: String,
    pub bound: String,
}

/// Every page under `directory`, at any depth, in path order.
///
/// # Errors
///
/// Returns an error if the directory cannot be walked.
pub fn pages_under(directory: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(pages_under(&path)?);
        } else if path.extension() == Some(OsStr::new(PAGE_EXTENSION)) {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// How a page names itself in a verdict.
///
/// The file name alone, never the path: a path renders with OS-specific
/// separators and an expectation carrying one would be a Windows-only or
/// Unix-only test.
#[must_use]
pub fn named(page: &Path) -> String {
    page.file_name()
        .unwrap_or_else(|| OsStr::new("a page with no name"))
        .to_string_lossy()
        .into_owned()
}

/// The cells of one markdown table row, trimmed.
#[must_use]
pub fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

/// Whether `line` is the header row of the table that names every field.
#[must_use]
pub fn is_the_field_table_header(line: &str) -> bool {
    let header = cells(line);
    header.first().is_some_and(|first| first == FIELD_COLUMN)
        && header.iter().any(|cell| cell == ABSENT_MEANS_COLUMN)
        && header.iter().any(|cell| cell == BOUND_COLUMN)
}

/// Every field the table starting at `header` names, beside what it says leaving
/// each out means, in the order the table names them.
fn tabulated(header: &str, rows: &mut dyn Iterator<Item = &str>) -> Vec<TabulatedRow> {
    let heading = cells(header);
    let at = |wanted: &str| heading.iter().position(|cell| cell == wanted);
    let (Some(absent_at), Some(bound_at)) = (at(ABSENT_MEANS_COLUMN), at(BOUND_COLUMN)) else {
        return Vec::new();
    };
    rows.take_while(|line| line.trim_start().starts_with('|'))
        .map(cells)
        .filter(|row| row.first().is_some_and(|first| first.starts_with('`')))
        .filter_map(|row| {
            Some(TabulatedRow {
                field: row.first()?.trim_matches('`').to_owned(),
                absent_means: row.get(absent_at)?.clone(),
                bound: row.get(bound_at)?.clone(),
            })
        })
        .collect()
}

/// Every table on `text` that names a declaration's fields beside what leaving
/// each out means.
#[must_use]
pub fn field_tables_in(text: &str) -> Vec<Vec<TabulatedRow>> {
    let mut tables = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if is_the_field_table_header(line) {
            // The alignment row between the header and the first field.
            lines.next();
            tables.push(tabulated(line, &mut lines));
        }
    }
    tables
}

/// Every such table under `directory`.
///
/// # Errors
///
/// Returns an error if the directory cannot be walked or a page cannot be read.
pub fn field_tables_under(directory: &Path) -> Result<Vec<Vec<TabulatedRow>>, Box<dyn Error>> {
    let mut tables = Vec::new();
    for page in pages_under(directory)? {
        tables.extend(field_tables_in(&fs::read_to_string(&page)?));
    }
    Ok(tables)
}

/// The one table under `directory` that names what a declaration may state.
///
/// # Errors
///
/// Returns an error unless exactly one page carries that table: none means the
/// reading has nothing to say and must not answer as though the table were
/// empty, and a second one means this reading would be silently choosing between
/// two statements of the same contract. A table whose `Bound` column has been
/// renamed is found by nothing and reaches the same error, which is the loud
/// failure rather than a silently empty bound.
pub fn the_field_table(directory: &Path) -> Result<Vec<TabulatedRow>, Box<dyn Error>> {
    let tables = field_tables_under(directory)?;
    let [table] = tables.as_slice() else {
        return Err(format!(
            "exactly one page under {} must tabulate what a declaration may state, and {} do",
            directory.display(),
            tables.len()
        )
        .into());
    };
    Ok(table.clone())
}

/// What the guide's table states in `column` for `field`, or `None` where it
/// carries no such row.
///
/// # Errors
///
/// Returns an error for the reason [`the_field_table`] does.
pub fn stated_for(
    directory: &Path,
    field: &str,
    column: fn(&TabulatedRow) -> String,
) -> Result<Option<String>, Box<dyn Error>> {
    Ok(the_field_table(directory)?
        .into_iter()
        .find(|row| row.field == field)
        .map(|row| column(&row)))
}

/// The `Bound` cell of a row.
#[must_use]
pub fn bound(row: &TabulatedRow) -> String {
    row.bound.clone()
}

/// The `Absent means` cell of a row.
#[must_use]
pub fn absent_means(row: &TabulatedRow) -> String {
    row.absent_means.clone()
}
