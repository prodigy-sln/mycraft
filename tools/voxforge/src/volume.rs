//! The assembled model: the part tree walked into one dense voxel volume.
//!
//! Everything downstream — the preview raycaster and the inspector — consumes a
//! [`Volume`] rather than a `Model`, so this is where the part tree stops being
//! a tree. The volume is a dense positional array rather than a map because the
//! ray march asks the same occupancy question hundreds of millions of times per
//! contact sheet, and a B-tree descent per question is the difference between a
//! preview loop an agent uses and one it abandons.
//!
//! **Coordinates.** A child's local voxel `p` occupies pre-normalisation
//! position `parent_position + attach.at + p − child.origin`. That is
//! legitimately negative: a pivot exists precisely so a part may extend in −x,
//! −y or −z from it. The assembled volume then translates the whole model —
//! never one part at a time — so that its lowest filled voxel sits at
//! `(0, 0, 0)`.

use std::collections::BTreeMap;
use std::num::NonZeroU16;

use glam::IVec3;

use crate::fault::Fault;
use crate::format::parse;
use crate::format::{Extent, FilledCell, Model, PartName, StateName, Voxel};
use crate::name::MaterialKey;

/// Which material fills one cell, as an index into the volume's own palette.
///
/// `NonZeroU16` rather than `u16` so that `Option<MaterialSlot>` is two bytes:
/// a 64-cube is 262 144 cells, and the inner loop of the ray march reads one of
/// them per step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MaterialSlot(NonZeroU16);

impl MaterialSlot {
    /// The slot standing for `index` into the palette, when one fits.
    fn at(index: usize) -> Option<Self> {
        let raised = index.checked_add(1)?;
        NonZeroU16::new(u16::try_from(raised).ok()?).map(Self)
    }

    /// The palette index this slot stands for.
    fn index(self) -> usize {
        usize::from(self.0.get().saturating_sub(1))
    }
}

/// One model, assembled: every part's art placed into one voxel space.
///
/// The minimum corner of the filled art is `(0, 0, 0)` by construction, which is
/// what [`Volume::filled_bounds`] re-derives from the cells rather than
/// remembering — a volume that is not tight has to be observable rather than
/// asserted away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// How far the assembled art reaches on each axis.
    extent: Extent,
    /// One entry per cell, indexed `x + y·ex + z·ex·ey`.
    cells: Vec<Option<MaterialSlot>>,
    /// Every material some cell holds, in ascending key order.
    palette: Vec<MaterialKey>,
    /// Where each part's local `(0, 0, 0)` landed, in assembled coordinates.
    placements: BTreeMap<PartName, IVec3>,
}

impl Volume {
    /// The volume of `extent` holding `filled`, with each part placed at
    /// `placements`.
    ///
    /// `filled` is already normalised: a cell outside `extent` is not a legal
    /// input and is dropped rather than wrapped, since wrapping would put art
    /// somewhere nobody wrote it.
    pub(crate) fn new(
        extent: Extent,
        filled: &[FilledCell],
        placements: BTreeMap<PartName, IVec3>,
    ) -> Self {
        let palette = palette_of(filled);
        let mut cells = vec![None; cell_count(extent)];
        for cell in filled {
            fill(&mut cells, extent, &palette, cell);
        }
        Self {
            extent,
            cells,
            palette,
            placements,
        }
    }

    /// How far the assembled art reaches on each axis.
    #[must_use]
    pub fn extent(&self) -> Extent {
        self.extent
    }

    /// Every filled voxel, ascending by position.
    #[must_use]
    pub fn filled(&self) -> Vec<FilledCell> {
        let mut cells: Vec<FilledCell> = self
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| self.cell_at(index, (*slot)?))
            .collect();
        cells.sort();
        cells
    }

    /// The inclusive lowest and highest corner the filled art occupies, or
    /// `None` when nothing at all is filled.
    ///
    /// Derived from the cells rather than remembered, so that a volume whose art
    /// does not reach its own minimum corner is visible rather than true by
    /// definition.
    #[must_use]
    pub fn filled_bounds(&self) -> Option<(Voxel, Voxel)> {
        let cells = self.filled();
        let mut positions = cells.iter().map(|cell| cell.position);
        let first = positions.next()?;
        Some(positions.fold((first, first), |(low, high), at| {
            (lowest(low, at), highest(high, at))
        }))
    }

    /// What fills the voxel at `position`, or `None` where nothing does.
    ///
    /// The one occupancy question the ray march asks, hundreds of millions of
    /// times per contact sheet — which is why the volume is a dense positional
    /// array rather than a map, and why this is an integer index rather than a
    /// tree descent.
    #[must_use]
    pub fn material_at(&self, position: Voxel) -> Option<&MaterialKey> {
        let slot = (*self.cells.get(index_of(self.extent, position)?)?)?;
        self.palette.get(slot.index())
    }

    /// Where `part`'s local voxel `local` landed, in assembled coordinates.
    ///
    /// Answers for any local coordinate, whether or not the part's art reaches
    /// it: an attachment point is routinely outside the extent of the part it
    /// hangs off, which is the whole reason a pivot exists.
    #[must_use]
    pub fn placed(&self, part: &PartName, local: Voxel) -> Option<IVec3> {
        let base = self.placements.get(part)?;
        Some(*base + offset_of(local)?)
    }

    /// The cell at `index`, when its slot names a material this volume holds.
    fn cell_at(&self, index: usize, slot: MaterialSlot) -> Option<FilledCell> {
        Some(FilledCell {
            position: self.position_of(index)?,
            material: self.palette.get(slot.index())?.clone(),
        })
    }

    /// The position `index` stands for.
    fn position_of(&self, index: usize) -> Option<Voxel> {
        let ex = usize::try_from(self.extent.x).ok()?;
        let ey = usize::try_from(self.extent.y).ok()?;
        let plane = ex.checked_mul(ey)?;
        let within = index.checked_rem(plane)?;
        Some(Voxel {
            x: u32::try_from(within.checked_rem(ex)?).ok()?,
            y: u32::try_from(within.checked_div(ex)?).ok()?,
            z: u32::try_from(index.checked_div(plane)?).ok()?,
        })
    }
}

/// Which state each part is assembled in.
///
/// Empty is the default and means every part takes the first state it declares,
/// which is what makes a document with states renderable without anyone naming
/// one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateSelection(BTreeMap<PartName, StateName>);

impl StateSelection {
    /// The same selection, with `part` taking `state`.
    #[must_use]
    pub fn with(mut self, part: PartName, state: StateName) -> Self {
        self.0.insert(part, state);
        self
    }

    /// The state `part` was asked for, when one was.
    #[must_use]
    pub fn get(&self, part: &PartName) -> Option<&StateName> {
        self.0.get(part)
    }
}

/// The volume `model` assembles to under `states`.
///
/// # Errors
///
/// Returns a [`Fault`] naming the axis and the assembled extent when the placed
/// parts exceed the 64-voxel limit on any axis, naming the state and the ones
/// the part declares when `states` asks for one it does not, and naming the
/// model when the assembled art holds no filled voxel at all.
pub fn assemble(model: &Model, states: &StateSelection) -> Result<Volume, Fault> {
    let placed = parse::assemble::place(model, states)?;
    let (lowest, reach) = parse::assemble::extent_of(&placed.cells, model)?;

    // One translation for the whole model, never one per part: a part reaching
    // below the model's lowest voxel drags every other part up with it, and
    // normalising each part onto its own zero would collapse exactly the
    // relative placement the attachment arithmetic just established.
    let filled: Vec<FilledCell> = placed
        .cells
        .into_iter()
        .filter_map(|(at, cell)| normalised(at - lowest, cell))
        .collect();
    let placements = placed
        .placements
        .into_iter()
        .map(|(part, at)| (part, at - lowest))
        .collect();

    Ok(Volume::new(extent_from(reach), &filled, placements))
}

/// One placed cell, at the position it occupies once the model is normalised.
fn normalised(at: IVec3, cell: FilledCell) -> Option<FilledCell> {
    Some(FilledCell {
        position: Voxel {
            x: u32::try_from(at.x).ok()?,
            y: u32::try_from(at.y).ok()?,
            z: u32::try_from(at.z).ok()?,
        },
        material: cell.material,
    })
}

/// How far the art reaches, as an extent.
fn extent_from(reach: IVec3) -> Extent {
    Extent {
        x: u32::try_from(reach.x).unwrap_or(0),
        y: u32::try_from(reach.y).unwrap_or(0),
        z: u32::try_from(reach.z).unwrap_or(0),
    }
}

/// Every material `filled` holds, ascending and without repeats.
fn palette_of(filled: &[FilledCell]) -> Vec<MaterialKey> {
    let mut keys: Vec<MaterialKey> = filled.iter().map(|cell| cell.material.clone()).collect();
    keys.sort();
    keys.dedup();
    keys
}

/// Writes `cell` into the position it occupies in a volume of `extent`.
///
/// A cell outside the extent, or of a material the palette does not hold, is
/// dropped: both are impossible for a normalised assembly, and wrapping one
/// round would put art somewhere nobody wrote it.
fn fill(
    cells: &mut [Option<MaterialSlot>],
    extent: Extent,
    palette: &[MaterialKey],
    cell: &FilledCell,
) {
    let slot = palette
        .binary_search(&cell.material)
        .ok()
        .and_then(MaterialSlot::at);
    let place = index_of(extent, cell.position).and_then(|index| cells.get_mut(index));
    if let (Some(slot), Some(place)) = (slot, place) {
        *place = Some(slot);
    }
}

/// How many cells a volume of `extent` holds.
fn cell_count(extent: Extent) -> usize {
    let axes = [extent.x, extent.y, extent.z];
    axes.into_iter()
        .try_fold(1_usize, |total, axis| {
            total.checked_mul(usize::try_from(axis).ok()?)
        })
        .unwrap_or(0)
}

/// The cell index `position` occupies in a volume of `extent`.
fn index_of(extent: Extent, position: Voxel) -> Option<usize> {
    if position.x >= extent.x || position.y >= extent.y || position.z >= extent.z {
        return None;
    }
    let ex = usize::try_from(extent.x).ok()?;
    let ey = usize::try_from(extent.y).ok()?;
    let x = usize::try_from(position.x).ok()?;
    let y = usize::try_from(position.y).ok()?;
    let z = usize::try_from(position.z).ok()?;
    let row = y.checked_mul(ex)?;
    let plane = z.checked_mul(ex)?.checked_mul(ey)?;
    x.checked_add(row)?.checked_add(plane)
}

/// A local voxel as a displacement from a part's own `(0, 0, 0)`.
fn offset_of(local: Voxel) -> Option<IVec3> {
    Some(IVec3::new(
        i32::try_from(local.x).ok()?,
        i32::try_from(local.y).ok()?,
        i32::try_from(local.z).ok()?,
    ))
}

/// The corner no higher than either of `left` and `right` on any axis.
fn lowest(left: Voxel, right: Voxel) -> Voxel {
    Voxel {
        x: left.x.min(right.x),
        y: left.y.min(right.y),
        z: left.z.min(right.z),
    }
}

/// The corner no lower than either of `left` and `right` on any axis.
fn highest(left: Voxel, right: Voxel) -> Voxel {
    Voxel {
        x: left.x.max(right.x),
        y: left.y.max(right.y),
        z: left.z.max(right.z),
    }
}
