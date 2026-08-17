//! Re-meshing what an edit changed, and putting it back where it came from.
//!
//! **The order the prepared sections sit in is the contract**, exactly as it is
//! for the whole-world mesh next door, and this is the file that could break it
//! without anything looking wrong. Every committed golden frame depends on the
//! order quads reach the packer in, so a re-mesh replaces entries *in place* and
//! is not allowed to append, to sort, or to grow the list: a section that was
//! third before an edit is third after it.
//!
//! That works because [`mesh_all`](super::prepare::mesh_all) emits an entry for
//! every section a column stacks, all-air ones included — a column holds a fixed
//! array of them, so nothing is filtered out. Were that ever to stop being true,
//! placing a block in a previously-empty section would need an append, an append
//! is a reorder, and the failure would arrive as a golden diff nobody would think
//! to look here for.
//!
//! A failed batch is the opposite of a failed preparation: preparation fails the
//! run because half a world is not a picture anybody should be shown, and a
//! re-mesh drops the batch and keeps the picture it already had. Neither of these
//! runs on the tick thread or the frame thread.

use mc_world::column::ColumnCoordinate;
use thiserror::Error;

use crate::world::{RemeshWork, SectionKey};

use super::prepare::{PrepareError, SectionQuads, SectionWork, around, mesh_one, section_origin};

/// Why a re-meshed section could not be put back.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpliceError {
    #[error(
        "section {section_index} of the column at ({x}, {z}) was re-meshed, but the prepared \
         scene holds no such section to replace",
        x = column.x, z = column.z
    )]
    NoSuchSection {
        column: ColumnCoordinate,
        section_index: usize,
    },
}

/// Every section `work` names, meshed against the neighbours it carries.
///
/// A key whose section the batch does not hold produces no entry: it is a
/// section the world does not stack, which is the same absence the whole-world
/// mesh answers with and not a failure.
///
/// # Errors
///
/// Returns [`PrepareError::Mesh`] naming the first section that could not be
/// meshed.
pub fn remesh(work: &RemeshWork) -> Result<Vec<SectionQuads>, PrepareError> {
    work.keys()
        .filter_map(|key| section_work(work, key))
        .map(|section| mesh_one(&section, work.registry()))
        .collect()
}

/// One section of a batch, and the six around it resolved out of that same
/// batch.
///
/// The neighbour arithmetic and the origin are the preparation's own, called
/// rather than re-derived — a re-mesh that placed a section one block off from
/// where the initial mesh put it would draw a world subtly sheared, and only
/// where somebody had dug.
fn section_work(work: &RemeshWork, key: SectionKey) -> Option<SectionWork<'_>> {
    Some(SectionWork {
        column: key.column,
        section_index: key.index,
        origin: section_origin(key.column, key.index),
        section: work.section(key)?,
        neighbours: around(
            |column, index| work.section(SectionKey { column, index }),
            key.column,
            key.index,
        ),
    })
}

/// Replaces each of `prepared`'s sections with the re-meshed one carrying the
/// same key.
///
/// **Positional, and a key it cannot find is an error rather than an append.**
/// The whole reason a re-mesh does not disturb the golden frames is that it
/// changes what is at a position and never which position anything is at, and an
/// append would be exactly the reorder that claim rules out. The parameter is a
/// slice for the same reason: appending is not merely forbidden here, it is not
/// expressible.
///
/// # Errors
///
/// Returns [`SpliceError::NoSuchSection`] naming the first re-meshed section the
/// prepared scene has no place for.
pub fn splice(
    prepared: &mut [SectionQuads],
    remeshed: Vec<SectionQuads>,
) -> Result<(), SpliceError> {
    for section in remeshed {
        let slot = prepared
            .iter_mut()
            .find(|entry| {
                entry.column == section.column && entry.section_index == section.section_index
            })
            .ok_or(SpliceError::NoSuchSection {
                column: section.column,
                section_index: section.section_index,
            })?;
        *slot = section;
    }
    Ok(())
}
