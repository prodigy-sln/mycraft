//! The `.mcvox` document: what one holds, and how one is read.
//!
//! A document is layered grid art. Each layer is a plane of the model, printed
//! as text, and the slice axis decides which plane and which way round the rows
//! and columns run. That mapping is the highest-risk decision in this tool — a
//! transposed or mirrored reading produces a *plausible* model rather than an
//! obviously broken one — so it lives behind one accessor,
//! [`Part::filled_cells`], and is graded there.

// The DTO layer is shared with `material`, which reads its own files through
// the same `deny_unknown_fields` discipline.
pub(crate) mod dto;
mod palette;
pub(crate) mod parse;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroU32;
use std::path::Path;

use glam::IVec3;

use crate::fault::{Fault, Origin};
use crate::material::MaterialTable;
use crate::name::{MaterialKey, ModelName};

/// The axis a document, or one of its parts, is sliced along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    /// Layers are `x` planes, read left to right.
    X,
    /// Layers are `y` planes, read upward from the ground.
    Y,
    /// Layers are `z` planes, read front to back.
    Z,
}

impl Axis {
    /// The axis as a document spells it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }
}

/// How far a part reaches on each axis, in voxels.
///
/// Three named fields rather than an array, because every refusal about an
/// extent has to name the *axis* at fault and an index cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Extent {
    /// Voxels on `x`.
    pub x: u32,
    /// Voxels on `y`.
    pub y: u32,
    /// Voxels on `z`.
    pub z: u32,
}

/// One voxel's position, in the local space of the part that declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Voxel {
    /// Position on `x`.
    pub x: u32,
    /// Position on `y`.
    pub y: u32,
    /// Position on `z`.
    pub z: u32,
}

/// The name a part is declared under. Not namespaced: a part is internal to the
/// document that declares it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartName(String);

impl PartName {
    /// The part named `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The name exactly as it was written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One of the states a part declares.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateName(String);

impl StateName {
    /// The state named `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The name exactly as it was written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What one palette character means.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PaletteEntry {
    /// No voxel at all.
    Empty,
    /// A voxel of this material.
    Material(MaterialKey),
}

/// Which layer of a part a grid belongs to.
///
/// `state` is `None` for a part that declares no states, rather than an empty
/// state name: "this part has no states" and "this part has a state spelled with
/// nothing" are different facts and a sentinel would let one arrive under the
/// other's name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerKey {
    /// The state this layer belongs to, or `None` for a stateless part.
    pub state: Option<StateName>,
    /// The plane the layer occupies along its part's slice axis.
    pub plane: u32,
}

/// One layer's art, after the palette has been applied.
///
/// Held as printed — `cell(row, column)` is the character at that position in
/// the document — so that the row-and-column-to-voxel mapping stays in one place
/// rather than being baked in twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    rows: Vec<Vec<Option<MaterialKey>>>,
}

impl Grid {
    /// A grid of `rows`, each cell already resolved against the palette and
    /// `None` where the palette entry is the empty marker.
    pub fn new(rows: Vec<Vec<Option<MaterialKey>>>) -> Self {
        Self { rows }
    }

    /// How many rows the grid holds.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many columns the grid's first row holds.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }

    /// The material at `row` and `column`, or `None` where the cell is empty or
    /// out of range.
    #[must_use]
    pub fn cell(&self, row: usize, column: usize) -> Option<&MaterialKey> {
        self.rows.get(row)?.get(column)?.as_ref()
    }
}

/// One voxel a part declares, and what it is made of.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FilledCell {
    /// Where the voxel sits in its part's local space.
    pub position: Voxel,
    /// What the voxel is made of.
    pub material: MaterialKey,
}

/// Where a part hangs off its parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attach {
    /// The part this one hangs off.
    pub to: PartName,
    /// The parent-local position this part's origin lands on.
    pub at: IVec3,
}

/// One part of a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The part's name, synthesised for the implicit single-part form.
    pub name: PartName,
    /// How far the part reaches on each axis.
    pub size: Extent,
    /// The part's pivot, in its own local space.
    pub origin: IVec3,
    /// The axis this part's layers are planes of — the model's default, or this
    /// part's own override.
    pub slice: Axis,
    /// Where the part hangs off its parent. `None` on exactly one part: the
    /// root.
    pub attach: Option<Attach>,
    /// The states this part declares. Empty means stateless; the first declared
    /// is the default.
    pub states: Vec<StateName>,
    /// The part's layers.
    pub layers: BTreeMap<LayerKey, Grid>,
}

impl Part {
    /// Every filled cell this part declares, in its own local voxel space,
    /// ascending by position.
    ///
    /// This is where the orientation contract is observable: the slice axis
    /// decides which plane a layer is and which way its rows and columns run.
    #[must_use]
    pub fn filled_cells(&self) -> Vec<FilledCell> {
        let mut cells = Vec::new();
        for (key, grid) in &self.layers {
            let reading = LayerReading {
                part: self,
                plane: key.plane,
                grid,
            };
            reading.collect_into(&mut cells);
        }
        cells.sort();
        cells
    }

    /// Every filled cell belonging to `state`, in this part's own local voxel
    /// space, ascending by position.
    ///
    /// `None` selects the layers of a stateless part. A state's art *replaces*
    /// another's rather than adding to it, which is what makes a flicker
    /// different voxels rather than a transform.
    #[must_use]
    pub fn filled_cells_in(&self, state: Option<&StateName>) -> Vec<FilledCell> {
        let mut cells = Vec::new();
        let belonging = self
            .layers
            .iter()
            .filter(|(key, _)| key.state.as_ref() == state);
        for (key, grid) in belonging {
            let reading = LayerReading {
                part: self,
                plane: key.plane,
                grid,
            };
            reading.collect_into(&mut cells);
        }
        cells.sort();
        cells
    }

    /// The state this part assembles in when nobody names one: the first it
    /// declares, or none at all when it is stateless.
    #[must_use]
    pub fn default_state(&self) -> Option<&StateName> {
        self.states.first()
    }
}

/// One layer of one part, being placed into voxel space.
struct LayerReading<'a> {
    /// The part whose slice axis and extent decide where the art lands.
    part: &'a Part,
    /// The plane the layer occupies along that axis.
    plane: u32,
    /// The art itself.
    grid: &'a Grid,
}

impl LayerReading<'_> {
    /// Every filled cell of this layer.
    fn collect_into(&self, cells: &mut Vec<FilledCell>) {
        for row in 0..self.grid.row_count() {
            let found = (0..self.grid.column_count()).filter_map(|column| self.cell(row, column));
            cells.extend(found);
        }
    }

    /// The voxel one grid position stands for, where it holds a material.
    ///
    /// This is the orientation contract, in the one place it exists. The three
    /// arms are the spec's own table, and they are deliberately together: a
    /// swapped row and a swapped column are the same mistake, and reading them
    /// side by side is what makes that visible.
    fn cell(&self, row: usize, column: usize) -> Option<FilledCell> {
        let material = self.grid.cell(row, column)?.clone();
        let row = u32::try_from(row).ok()?;
        let column = u32::try_from(column).ok()?;
        let position = match self.part.slice {
            Axis::Y => Voxel {
                x: column,
                y: self.plane,
                z: row,
            },
            Axis::Z => Voxel {
                x: column,
                y: self.descending(row)?,
                z: self.plane,
            },
            Axis::X => Voxel {
                x: self.plane,
                y: self.descending(row)?,
                z: column,
            },
        };
        Some(FilledCell { position, material })
    }

    /// Which `y` a printed row stands for, on a layer that prints `y`
    /// descending — the first row printed is the *top* of the model.
    ///
    /// Computed per arm rather than once above the match, and that is not a
    /// tidiness point. A `y` layer's rows run along `z`, so on a part whose `z`
    /// extent exceeds its `y` extent this subtraction underflows for rows the
    /// `y` arm never needed it for — and an underflow here answers `None`, which
    /// silently *drops the voxel* rather than failing. Every `y`-sliced part
    /// with more depth than height lost art that way.
    fn descending(&self, row: u32) -> Option<u32> {
        self.part.size.y.checked_sub(1)?.checked_sub(row)
    }
}

/// A model document, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The namespaced name the document declares.
    pub name: ModelName,
    /// How many voxels span one block edge.
    pub scale: NonZeroU32,
    /// The document's parts, in declaration order.
    pub parts: Vec<Part>,
    /// The palette, keyed by the ASCII byte a grid spells an entry with.
    pub palette: BTreeMap<u8, PaletteEntry>,
    origin: Origin,
    /// Every palette character some grid of this model actually spells.
    ///
    /// Kept rather than recomputed, because the art no longer remembers it: a
    /// resolved grid holds `None` for every empty cell, so an entry mapping to
    /// the empty marker and an entry nothing spells look identical there.
    spelled: BTreeSet<u8>,
}

impl Model {
    /// The file this model was read from.
    #[must_use]
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The part declared under `name`.
    #[must_use]
    pub fn part(&self, name: &str) -> Option<&Part> {
        self.parts.iter().find(|part| part.name.as_str() == name)
    }

    /// The model's height in blocks: the tallest declared part extent on `y`
    /// over the declared scale.
    ///
    /// A declared-extent answer, not an assembled one — assembly places parts
    /// relative to one another and is a later question than this one.
    #[must_use]
    pub fn height_in_blocks(&self) -> f64 {
        let tallest = self.parts.iter().map(|part| part.size.y).max().unwrap_or(0);
        f64::from(tallest) / f64::from(self.scale.get())
    }

    /// Every palette key no grid of this model uses, ascending.
    ///
    /// An unused entry is a defect the inspector grades; loading is not the
    /// place it is refused.
    #[must_use]
    pub fn unused_palette_keys(&self) -> Vec<u8> {
        self.palette
            .keys()
            .copied()
            .filter(|key| !self.spelled.contains(key))
            .collect()
    }

    /// Checks that every material this model's palette names is declared by
    /// `materials`.
    ///
    /// # Errors
    ///
    /// Returns a [`Fault`] naming the key and the directory searched, if the
    /// palette names a material the table does not declare.
    pub fn bind_materials(&self, materials: &MaterialTable) -> Result<(), Fault> {
        self.palette
            .values()
            .filter_map(|entry| match entry {
                PaletteEntry::Empty => None,
                PaletteEntry::Material(key) => Some(key),
            })
            .find(|key| materials.get(key).is_none())
            .map_or(Ok(()), |key| Err(self.unresolved(key, materials)))
    }

    /// The refusal a palette entry nothing declares earns.
    ///
    /// Names the directory as well as the key: the author's likeliest repair is
    /// to a `--materials` path rather than to the palette, and a refusal naming
    /// only the key sends them to the wrong file.
    fn unresolved(&self, key: &MaterialKey, materials: &MaterialTable) -> Fault {
        Fault::about(
            self.origin.clone(),
            format!(
                "the palette names the material `{key}`, which no file in {directory} declares",
                key = key.as_str(),
                directory = materials.directory().display()
            ),
        )
        .in_field("palette")
    }
}

/// The model the document at `path` describes.
///
/// # Errors
///
/// Returns a [`Fault`] naming `path` if the file cannot be read, or the refusal
/// [`parse_document`] makes.
pub fn load_document(path: &Path) -> Result<Model, Fault> {
    let origin = Origin::new(path);
    let text = fs::read_to_string(path)
        .map_err(|cause| Fault::about(origin.clone(), cause.to_string()))?;
    parse_document(&text, origin)
}

/// The model `text` describes, attributed to `origin`.
///
/// # Errors
///
/// Returns a [`Fault`] naming the origin, the element and the field at fault if
/// the document is not a legal `.mcvox` document.
pub fn parse_document(text: &str, origin: Origin) -> Result<Model, Fault> {
    parse::document(text, origin)
}
