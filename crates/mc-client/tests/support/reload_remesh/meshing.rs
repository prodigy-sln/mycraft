//! What meshing a batch came to, and how many of a block's faces the result shows.
//!
//! The batch is drained through [`super`], so a scenario reading it here has read
//! it once and cannot ask again.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use mc_core::block::BlockRegistry;
use mc_render::window::rendered;
use mc_sim::replay::{SectionQuads, remesh};
use mc_sim::world::{RemeshWork, World};
use mc_world::mesh::{Facing, Quad};
use mc_world::world::VoxelWorld;

use crate::input::InputHarness;

use super::Section;

/// One section as it was meshed: where its near corner sits, and the faces it
/// shows.
///
/// The origin travels with the quads because a re-mesh that placed a section one
/// block from where the whole-world mesh put it would draw a world subtly sheared,
/// and a comparison over the quads alone could not see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshedSection {
    pub origin: [i32; 3],
    pub quads: Vec<Quad>,
}

/// Every section a mesh produced, keyed by which section it is.
pub type Sections = BTreeMap<Section, MeshedSection>;

/// What meshing a batch came to.
///
/// **A total verdict and never a propagated error**: a batch that refused to mesh
/// fails the comparison naming what it said instead of ending the test before its
/// assertion ran, and a batch that never existed has an arm rather than arriving
/// as an empty map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Meshed {
    /// Every section the batch named, meshed.
    Sections(Sections),
    /// The batch could not be meshed, and this is what it said.
    Refused { said: String },
    /// Nothing was left to mesh, so there was no batch.
    NoBatch,
}

/// What meshing whatever `client` was left to mesh came to, taken once.
///
/// **The batch is meshed through no registry of this fixture's own.** A batch
/// carries the registry the world that produced it was resolved against, so a
/// section meshed here is meshed against the content the client is serving and
/// there is no second opinion to hand in.
pub fn meshed(client: &mut InputHarness) -> Meshed {
    let Some(work) = client.take_remesh_work() else {
        return Meshed::NoBatch;
    };
    meshed_of(&work)
}

/// The same, for a batch a scenario is holding.
#[must_use]
pub fn meshed_of(work: &RemeshWork) -> Meshed {
    match remesh(work) {
        Ok(sections) => Meshed::Sections(as_sections(sections)),
        Err(refused) => Meshed::Refused {
            said: rendered(&refused),
        },
    }
}

/// Every section of `blocks`, meshed against `registry`.
///
/// The independent oracle a batch's own meshing is compared against: it shares no
/// batch, no dirty set and no registry with the client under test, and it is the
/// same whole-world mesh a launch produces.
///
/// # Errors
///
/// Returns an error if the world does not resolve against `registry`, or if it
/// cannot be meshed.
pub fn meshed_against(
    blocks: VoxelWorld,
    registry: Arc<BlockRegistry>,
) -> Result<Meshed, Box<dyn Error>> {
    Ok(Meshed::Sections(as_sections(
        World::new(blocks, registry)?.mesh()?,
    )))
}

/// A meshed list keyed by which section each entry is.
fn as_sections(meshed: Vec<SectionQuads>) -> Sections {
    meshed
        .into_iter()
        .map(|section| {
            (
                (section.column.x, section.column.z, section.section_index),
                MeshedSection {
                    origin: section.origin,
                    quads: section.quads,
                },
            )
        })
        .collect()
}

/// How many of a block's faces a mesh shows, or what that mesh came to instead.
///
/// **A total verdict**, so a count of zero cannot stand in for a batch that
/// refused or one that never existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Faces {
    Showing(usize),
    Refused { said: String },
    NoBatch,
}

/// How many faces `meshed` shows for `block`.
#[must_use]
pub fn faces_shown(meshed: &Meshed, block: &str) -> Faces {
    counting(meshed, |sections| faces_of(sections, block))
}

/// How many of `block`'s faces in `meshed` point along `facing`.
#[must_use]
pub fn faces_shown_facing(meshed: &Meshed, block: &str, facing: Facing) -> Faces {
    counting(meshed, |sections| faces_facing(sections, block, facing))
}

/// Whatever `count` makes of a mesh's sections, or the arm the mesh came to.
fn counting(meshed: &Meshed, count: impl FnOnce(&Sections) -> usize) -> Faces {
    match meshed {
        Meshed::Sections(sections) => Faces::Showing(count(sections)),
        Meshed::Refused { said } => Faces::Refused { said: said.clone() },
        Meshed::NoBatch => Faces::NoBatch,
    }
}

/// How many faces `sections` shows for `block`.
#[must_use]
pub fn faces_of(sections: &Sections, block: &str) -> usize {
    quads_of(sections, block).count()
}

/// How many of `block`'s faces in `sections` point along `facing`.
#[must_use]
pub fn faces_facing(sections: &Sections, block: &str, facing: Facing) -> usize {
    quads_of(sections, block)
        .filter(|quad| quad.facing == facing)
        .count()
}

/// Every quad of `sections` that holds `block`.
fn quads_of<'a>(sections: &'a Sections, block: &'a str) -> impl Iterator<Item = &'a Quad> {
    sections
        .values()
        .flat_map(|section| section.quads.iter())
        .filter(move |quad| quad.block.as_str() == block)
}

/// Whatever a mesh came to as its sections, or an error naming what it came to
/// instead.
///
/// For the guards that need the sections themselves rather than a verdict.
///
/// # Errors
///
/// Returns an error unless the batch was meshed.
pub fn sections_meshed(meshed: Meshed) -> Result<Sections, Box<dyn Error>> {
    match meshed {
        Meshed::Sections(sections) => Ok(sections),
        other => Err(format!(
            "this fixture needs the batch to have been meshed before it can read faces out of it, \
             and it came to {other:?}"
        )
        .into()),
    }
}
