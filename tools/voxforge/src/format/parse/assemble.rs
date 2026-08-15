//! Walking the part tree into one voxel space.
//!
//! D9's third pass. Two things happen here that cannot happen earlier: a part's
//! position depends on its parent's, so nothing is placed until the whole chain
//! above it is; and the assembled bounding box is a property of where the art
//! *landed*, which no declared `size` can answer.
//!
//! **The root's own `origin` is deliberately not applied.** Both readings —
//! root at zero, or root translated by `−origin` — shift the entire model
//! uniformly, and normalisation removes a uniform shift, so the two are
//! indistinguishable in every assembled coordinate. Zero is chosen because it
//! keeps a single-part model's placement equal to its art.

use std::collections::BTreeMap;

use glam::IVec3;

use crate::fault::Fault;
use crate::format::parse::state::quoted;
use crate::format::{FilledCell, Model, Part, PartName, Voxel};
use crate::volume::StateSelection;

/// The most voxels an assembled model may reach on any one axis.
const MAX_EXTENT: i32 = 64;

/// One part's art, already placed in the model's pre-normalisation space.
#[derive(Debug)]
pub struct Placed {
    /// Where each part's local `(0, 0, 0)` landed.
    pub placements: BTreeMap<PartName, IVec3>,
    /// Every filled voxel of every part, in pre-normalisation coordinates.
    pub cells: Vec<(IVec3, FilledCell)>,
}

/// Places every part of `model` under `states`, before normalisation.
///
/// # Errors
///
/// Returns a [`Fault`] naming the state and the ones a part declares when
/// `states` asks for one it does not.
pub fn place(model: &Model, states: &StateSelection) -> Result<Placed, Fault> {
    check_requested_states(model, states)?;
    let placements = placements_of(model);
    let mut cells = Vec::new();
    for part in &model.parts {
        let Some(base) = placements.get(&part.name) else {
            continue;
        };
        collect(part, *base, states, &mut cells);
    }
    Ok(Placed { placements, cells })
}

/// Where the assembled art starts, and how far it reaches from there.
///
/// Both are measured on the art rather than on any declared `size`: a part may
/// legitimately declare more extent than it fills, and the 64-voxel limit is
/// about the box the model actually occupies.
///
/// # Errors
///
/// Returns a [`Fault`] naming the model when nothing at all is filled, and one
/// naming the axis and the assembled figure when the art reaches past
/// [`MAX_EXTENT`].
pub fn extent_of(cells: &[(IVec3, FilledCell)], model: &Model) -> Result<(IVec3, IVec3), Fault> {
    let mut positions = cells.iter().map(|(at, _)| *at);
    let first = positions.next().ok_or_else(|| nothing_filled(model))?;
    let (lowest, highest) = positions.fold((first, first), |(low, high), at| {
        (low.min(at), high.max(at))
    });
    let reach = highest - lowest + IVec3::ONE;
    for (axis, span) in [("x", reach.x), ("y", reach.y), ("z", reach.z)] {
        if span > MAX_EXTENT {
            return Err(Fault::about(
                model.origin().clone(),
                format!(
                    "the assembled model reaches {span} voxels on axis {axis}, but the limit is {MAX_EXTENT} voxels on any axis"
                ),
            ));
        }
    }
    Ok((lowest, reach))
}

/// The refusal a model whose art never landed earns.
///
/// A defect rather than a legitimate empty answer: every path to it — a palette
/// of nothing but the empty marker, art that is all empty cells, a part whose
/// layers were never written — is something the author meant to fill.
fn nothing_filled(model: &Model) -> Fault {
    Fault::about(
        model.origin().clone(),
        format!(
            "the model `{name}` assembles to no filled voxel at all",
            name = model.name.as_str()
        ),
    )
}

/// Refuses a selection naming a state a part does not declare.
fn check_requested_states(model: &Model, states: &StateSelection) -> Result<(), Fault> {
    for part in &model.parts {
        let Some(asked) = states.get(&part.name) else {
            continue;
        };
        if !part.states.contains(asked) {
            return Err(Fault::about(
                model.origin().clone(),
                format!(
                    "the part `{name}` was asked for the state `{spelled}`, which it does not declare — it declares {declared}",
                    name = part.name.as_str(),
                    spelled = asked.as_str(),
                    declared = quoted(&part.states)
                ),
            )
            .in_part(part.name.as_str()));
        }
    }
    Ok(())
}

/// Where each part's local `(0, 0, 0)` lands, walking down from the root.
///
/// A part is placed only once its parent is, so the walk repeats until it stops
/// making progress. The tree check has already refused anything unreachable, so
/// every part is placed by the time it does.
fn placements_of(model: &Model) -> BTreeMap<PartName, IVec3> {
    let mut placements: BTreeMap<PartName, IVec3> = model
        .parts
        .iter()
        .filter(|part| part.attach.is_none())
        .map(|part| (part.name.clone(), IVec3::ZERO))
        .collect();
    let mut settled = placements.len();
    loop {
        for part in &model.parts {
            place_child(part, &mut placements);
        }
        if placements.len() == settled {
            return placements;
        }
        settled = placements.len();
    }
}

/// Places one part, if its parent is placed and it is not.
fn place_child(part: &Part, placements: &mut BTreeMap<PartName, IVec3>) {
    let Some(attach) = part.attach.as_ref() else {
        return;
    };
    if placements.contains_key(&part.name) {
        return;
    }
    let Some(parent) = placements.get(&attach.to).copied() else {
        return;
    };
    placements.insert(part.name.clone(), parent + attach.at - part.origin);
}

/// Every filled cell of `part`, displaced by where the part landed.
fn collect(
    part: &Part,
    base: IVec3,
    states: &StateSelection,
    cells: &mut Vec<(IVec3, FilledCell)>,
) {
    let state = states.get(&part.name).or_else(|| part.default_state());
    let found = part
        .filled_cells_in(state)
        .into_iter()
        .filter_map(|cell| Some((base + local_offset(cell.position)?, cell)));
    cells.extend(found);
}

/// A local voxel as a displacement from its part's own `(0, 0, 0)`.
fn local_offset(local: Voxel) -> Option<IVec3> {
    Some(IVec3::new(
        i32::try_from(local.x).ok()?,
        i32::try_from(local.y).ok()?,
        i32::try_from(local.z).ok()?,
    ))
}
