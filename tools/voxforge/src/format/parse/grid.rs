//! Reading a layer's art, and placing it on the right plane of the right part.
//!
//! A grid is checked against the extent its slice axis implies, exactly: a row
//! too many, a column too few, or a trailing space that no editor shows are all
//! refused rather than trimmed into agreement. Trimming is what turns an
//! invisible typo into a model that is quietly the wrong shape.

use std::collections::{BTreeMap, BTreeSet};

use toml::Value;

use crate::fault::{Fault, LayerRef, Origin};
use crate::format::dto::{DocumentDto, LayerDto, from_value};
use crate::format::{Axis, Grid, LayerKey, PaletteEntry, Part, StateName};
use crate::name::MaterialKey;

/// Reads every layer the document declares into the part it belongs to.
///
/// Answers with every palette character some grid spelled, which is what makes
/// an entry nothing spells observable without a second walk over the art.
///
/// # Errors
///
/// Returns a [`Fault`] naming the layer, and the row and column where one is at
/// fault, for a grid that does not fit its part, holds a character outside
/// ASCII, spells a character the palette does not declare, or repeats a plane.
pub fn read_layers(
    document: &DocumentDto,
    parts: &mut [Part],
    palette: &BTreeMap<u8, PaletteEntry>,
    origin: &Origin,
) -> Result<BTreeSet<u8>, Fault> {
    let mut spelled = BTreeSet::new();
    let Some(declared) = document.layers.as_ref().and_then(Value::as_array) else {
        return Ok(spelled);
    };
    for (declaration, value) in declared.iter().enumerate() {
        let layer: LayerDto = from_value(value.clone(), origin)
            .map_err(|fault| fault.in_layer(LayerRef::declared(declaration)))?;
        let at = Placement {
            declaration,
            origin,
            palette,
        };
        place(&layer, parts, at, &mut spelled)?;
    }
    Ok(spelled)
}

/// Where in the document a layer was declared, and what it is read against.
#[derive(Clone, Copy)]
struct Placement<'a> {
    /// Position of the layer's table in the document, counted from zero.
    declaration: usize,
    /// The document the layer was declared in.
    origin: &'a Origin,
    /// The palette every character is resolved against.
    palette: &'a BTreeMap<u8, PaletteEntry>,
}

/// Reads one layer and places it on the part that declares it.
fn place(
    layer: &LayerDto,
    parts: &mut [Part],
    at: Placement<'_>,
    spelled: &mut BTreeSet<u8>,
) -> Result<(), Fault> {
    let named = layer.part.as_ref().and_then(Value::as_str);
    let index = part_index(parts, named, at)?;
    let part = parts.get_mut(index).ok_or_else(|| {
        Fault::about(at.origin.clone(), "this layer belongs to no part")
            .in_layer(LayerRef::declared(at.declaration))
    })?;

    let mut reading = Reading::of(part, at, spelled);
    let plane = reading.plane(layer)?;
    let key = LayerKey {
        state: layer
            .state
            .as_ref()
            .and_then(Value::as_str)
            .map(StateName::new),
        plane,
    };
    if part.layers.contains_key(&key) {
        return Err(reading.repeated(plane));
    }
    let grid = reading.grid(layer)?;
    part.layers.insert(key, grid);
    Ok(())
}

/// Which part a layer belongs to.
fn part_index(parts: &[Part], named: Option<&str>, at: Placement<'_>) -> Result<usize, Fault> {
    let Some(name) = named else {
        return Ok(0);
    };
    parts
        .iter()
        .position(|part| part.name.as_str() == name)
        .ok_or_else(|| {
            Fault::about(
                at.origin.clone(),
                format!("this layer names the part `{name}`, which the document does not declare"),
            )
            .in_layer(LayerRef::declared(at.declaration))
        })
}

/// One layer being read, and everything a refusal about it has to name.
///
/// The shape the part's slice axis implies is settled once, here, rather than
/// re-derived per row: the rows-and-columns question and the which-plane
/// question are the same decision, and answering them apart is how they come to
/// disagree.
struct Reading<'a> {
    palette: &'a BTreeMap<u8, PaletteEntry>,
    origin: &'a Origin,
    declaration: usize,
    part: String,
    axis: Axis,
    extent: u32,
    rows: usize,
    columns: usize,
    spelled: &'a mut BTreeSet<u8>,
}

impl<'a> Reading<'a> {
    /// The reading of a layer belonging to `part`.
    fn of(part: &Part, at: Placement<'a>, spelled: &'a mut BTreeSet<u8>) -> Self {
        let (rows, columns) = match part.slice {
            Axis::X => (part.size.y, part.size.z),
            Axis::Y => (part.size.z, part.size.x),
            Axis::Z => (part.size.y, part.size.x),
        };
        let extent = match part.slice {
            Axis::X => part.size.x,
            Axis::Y => part.size.y,
            Axis::Z => part.size.z,
        };
        Self {
            palette: at.palette,
            origin: at.origin,
            declaration: at.declaration,
            part: part.name.as_str().to_owned(),
            axis: part.slice,
            extent,
            rows: rows as usize,
            columns: columns as usize,
            spelled,
        }
    }

    /// A refusal about this layer, identified by where it was declared.
    fn refusal(&self, cause: impl Into<String>) -> Fault {
        Fault::about(self.origin.clone(), cause)
            .in_part(self.part.clone())
            .in_layer(LayerRef::declared(self.declaration))
    }

    /// A refusal about this layer, identified by the plane it declared.
    fn refusal_at(&self, plane: i64, cause: impl Into<String>) -> Fault {
        let reference = i32::try_from(plane).map_or_else(
            |_| LayerRef::declared(self.declaration),
            |plane| LayerRef::at_plane(self.declaration, plane),
        );
        Fault::about(self.origin.clone(), cause)
            .in_part(self.part.clone())
            .in_layer(reference)
    }

    /// The refusal a plane declared twice earns.
    fn repeated(&self, plane: u32) -> Fault {
        self.refusal_at(
            i64::from(plane),
            format!(
                "two layers of this part declare {axis} = {plane}, which is ambiguous rather than additive",
                axis = self.axis.as_str()
            ),
        )
    }

    /// The plane this layer declares, checked against its part's extent.
    fn plane(&self, layer: &LayerDto) -> Result<u32, Fault> {
        let declared = match self.axis {
            Axis::X => layer.x.as_ref(),
            Axis::Y => layer.y.as_ref(),
            Axis::Z => layer.z.as_ref(),
        };
        let axis = self.axis.as_str();
        let plane = declared.and_then(Value::as_integer).ok_or_else(|| {
            self.refusal(format!(
                "this part is sliced on `{axis}`, so each of its layers declares the plane it occupies as `{axis} = <index>`"
            ))
        })?;
        let extent = i64::from(self.extent);
        if plane < 0 || plane >= extent {
            return Err(self.refusal_at(
                plane,
                format!(
                    "this layer declares {axis} = {plane}, but its part has an extent of {extent} on that axis, so its last plane is {last}",
                    last = extent - 1
                ),
            ));
        }
        u32::try_from(plane).map_err(|cause| self.refusal_at(plane, cause.to_string()))
    }

    /// The art this layer declares, resolved against the palette.
    fn grid(&mut self, layer: &LayerDto) -> Result<Grid, Fault> {
        let text = layer
            .grid
            .as_ref()
            .and_then(Value::as_str)
            .ok_or_else(|| self.refusal("every layer declares its art as a `grid`"))?;
        let lines = lines_of(text);
        if lines.len() != self.rows {
            return Err(self.refusal(format!(
                "this layer is expected {rows} rows by its part's extent, but found {found}",
                rows = self.rows,
                found = lines.len()
            )));
        }
        let mut resolved = Vec::with_capacity(lines.len());
        for (index, line) in lines.iter().enumerate() {
            resolved.push(self.row(index, line)?);
        }
        Ok(Grid::new(resolved))
    }

    /// One row of the art.
    fn row(&mut self, row: usize, text: &str) -> Result<Vec<Option<MaterialKey>>, Fault> {
        let spellings: Vec<char> = text.chars().collect();
        if spellings.len() != self.columns {
            return Err(self.refusal(format!(
                "row {row} of this layer is expected {columns} columns by its part's extent, but found {found}",
                columns = self.columns,
                found = spellings.len()
            )));
        }
        let mut resolved = Vec::with_capacity(spellings.len());
        for (column, spelling) in spellings.into_iter().enumerate() {
            resolved.push(self.cell(row, column, spelling)?);
        }
        Ok(resolved)
    }

    /// One cell of the art.
    fn cell(
        &mut self,
        row: usize,
        column: usize,
        spelling: char,
    ) -> Result<Option<MaterialKey>, Fault> {
        if !spelling.is_ascii() {
            return Err(self.refusal(format!(
                "row {row}, column {column} of this layer holds `{spelling}`, which is not ASCII — a palette key is one ASCII character"
            )));
        }
        let key = u8::try_from(spelling).map_err(|cause| self.refusal(cause.to_string()))?;
        let entry = self.palette.get(&key).ok_or_else(|| {
            self.refusal(format!(
                "row {row}, column {column} of this layer holds `{spelling}`, which the palette does not declare"
            ))
        })?;
        self.spelled.insert(key);
        Ok(match entry {
            PaletteEntry::Empty => None,
            PaletteEntry::Material(material) => Some(material.clone()),
        })
    }
}

/// The rows a grid's text holds.
///
/// A carriage return is a line ending and never a cell, and the single newline a
/// `"""` block carries before its closing delimiter is not a row — TOML has
/// already eaten the one after the opening delimiter.
fn lines_of(text: &str) -> Vec<&str> {
    let trimmed = text.strip_suffix('\n').unwrap_or(text);
    trimmed
        .split('\n')
        .map(|row| row.strip_suffix('\r').unwrap_or(row))
        .collect()
}
